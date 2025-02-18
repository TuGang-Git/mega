use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use libp2p::kad::{self, Quorum, Record};
use libp2p::{PeerId, Swarm};
use secp256k1::{rand, KeyPair, Secp256k1};
use tokio::sync::Mutex;

use common::utils;

use crate::get_utc_timestamp;
use crate::network::behaviour::{GitInfoRefsReq, GitUploadPackReq};
use crate::node::MegaRepoInfo;
use crate::nostr::client_message::{ClientMessage, Filter, SubscriptionId};
use crate::nostr::event::{GitEvent, NostrEvent};
use crate::nostr::NostrReq;
use crate::{get_pack_protocol, get_repo_full_path, network::behaviour, node::ClientParas};

pub struct CmdHandler {
    pub swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    pub client_paras: Arc<Mutex<ClientParas>>,
}

impl CmdHandler {
    pub async fn provide(&self, repo_name: &str) {
        {
            let client_paras = self.client_paras.lock().await;
            if !repo_name.ends_with(".git") {
                eprintln!("repo_name should end with .git");
                return;
            }
            let path = get_repo_full_path(repo_name);
            let pack_protocol: git::protocol::PackProtocol =
                get_pack_protocol(&path, client_paras.storage.clone());
            let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
            if object_id == *utils::ZERO_ID {
                eprintln!("Repository not found");
                return;
            }
            let mut swarm = self.swarm.lock().await;
            // //Construct repoInfo
            let mega_repo_info = MegaRepoInfo {
                origin: swarm.local_peer_id().to_string(),
                name: repo_name.to_string(),
                latest: object_id,
                forks: vec![],
                timestamp: get_utc_timestamp(),
            };
            let record = Record {
                key: kad::RecordKey::new(&repo_name),
                value: serde_json::to_vec(&mega_repo_info).unwrap(),
                publisher: None,
                expires: None,
            };

            if let Err(e) = swarm
                .behaviour_mut()
                .kademlia
                .put_record(record, Quorum::One)
            {
                eprintln!("Failed to store record:{}", e);
            }
        }
    }

    pub async fn search(&self, repo_name: &str) {
        {
            let mut swarm = self.swarm.lock().await;
            swarm
                .behaviour_mut()
                .kademlia
                .get_record(kad::RecordKey::new(&repo_name));
        }
    }

    pub async fn clone(&self, mega_address: &str) {
        // mega clone p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git

        let (peer_id, repo_name) = match parse_mega_address(mega_address) {
            Ok((peer_id, repo_name)) => (peer_id, repo_name),
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        let path = get_repo_full_path(repo_name);
        {
            let mut swarm = self.swarm.lock().await;
            let request_file_id = swarm.behaviour_mut().git_upload_pack.send_request(
                &peer_id,
                GitUploadPackReq(HashSet::new(), HashSet::new(), path),
            );
            let mut client_paras = self.client_paras.lock().await;
            client_paras
                .pending_git_upload_package
                .insert(request_file_id, repo_name.to_string());
        }
    }

    pub async fn pull(&self, mega_address: &str) {
        let (peer_id, repo_name) = match parse_mega_address(mega_address) {
            Ok((peer_id, repo_name)) => (peer_id, repo_name),
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        let path = get_repo_full_path(repo_name);
        let mut client_paras = self.client_paras.lock().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
        if object_id == *utils::ZERO_ID {
            eprintln!("local repo not found");
            return;
        }
        {
            // Request to get git_info_refs
            let mut swarm = self.swarm.lock().await;
            let request_id = swarm
                .behaviour_mut()
                .git_info_refs
                .send_request(&peer_id, GitInfoRefsReq(path, Vec::new()));
            client_paras
                .pending_git_pull
                .insert(request_id, repo_name.to_string());
        }
    }

    pub async fn clone_obj(&self, repo_name: &str) {
        if !repo_name.ends_with(".git") {
            eprintln!("repo_name should end with .git");
            return;
        }
        {
            let mut swarm = self.swarm.lock().await;
            let kad_query_id = swarm
                .behaviour_mut()
                .kademlia
                .get_record(kad::RecordKey::new(&repo_name));

            let mut client_paras = self.client_paras.lock().await;
            client_paras
                .pending_repo_info_search_to_download_obj
                .insert(kad_query_id, repo_name.to_owned());
        }
    }

    pub async fn pull_obj(&self, repo_name: &str) {
        if !repo_name.ends_with(".git") {
            eprintln!("repo_name should end with .git");
            return;
        }
        let mut swarm = self.swarm.lock().await;
        let kad_query_id = swarm
            .behaviour_mut()
            .kademlia
            .get_record(kad::RecordKey::new(&repo_name));
        {
            let mut client_paras = self.client_paras.lock().await;

            client_paras
                .pending_repo_info_search_to_download_obj
                .insert(kad_query_id, repo_name.to_owned());
        }
    }

    pub async fn subscribe(&self, repo_name: &str) {
        let client_paras = self.client_paras.lock().await;
        let relay_peer_id = client_paras.rendezvous_point.unwrap();
        let filters = vec![Filter::new().repo_name(repo_name.to_string())];
        let client_req = ClientMessage::new_req(SubscriptionId::generate(), filters);

        {
            let mut swarm = self.swarm.lock().await;
            swarm
                .behaviour_mut()
                .nostr
                .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
        }
    }

    pub async fn event_update(&self, repo_name: &str) {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);

        let mut swarm = self.swarm.lock().await;

        let peer_id = swarm.local_peer_id().to_string();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name);
        let client_paras = self.client_paras.lock().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id,
            repo_name: repo_name.to_string(),
            repo_target: "origin".to_string(),
            repo_action: "update".to_string(),
            repo_url: url,
            repo_commit_id: object_id,
            repo_issue_content: "".to_string(),
        };
        let event = NostrEvent::new_git_event(key_pair, git_event);

        let client_req = ClientMessage::new_event(event);
        let relay_peer_id = client_paras.rendezvous_point.unwrap();
        swarm
            .behaviour_mut()
            .nostr
            .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
    }

    pub async fn event_merge(&self, repo_name: &str) {
        let mut swarm = self.swarm.lock().await;

        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let peer_id = swarm.local_peer_id().to_string();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name);
        let client_paras = self.client_paras.lock().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id: swarm.local_peer_id().to_string(),
            repo_name: repo_name.to_string(),
            repo_target: "fork".to_string(),
            repo_action: "request".to_string(),
            repo_url: url,
            repo_commit_id: object_id,
            repo_issue_content: "".to_string(),
        };
        let event = NostrEvent::new_git_event(key_pair, git_event);

        let client_req = ClientMessage::new_event(event);
        let relay_peer_id = client_paras.rendezvous_point.unwrap();
        swarm
            .behaviour_mut()
            .nostr
            .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
    }

    pub async fn event_issue(&self, repo_name: &str) {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let swarm = &mut self.swarm.lock().await;
        let peer_id = swarm.local_peer_id().to_string();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name);
        let client_paras = self.client_paras.lock().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id: swarm.local_peer_id().to_string(),
            repo_name: repo_name.to_string(),
            repo_target: "fork".to_string(),
            repo_action: "issue".to_string(),
            repo_url: url,
            repo_commit_id: object_id,
            repo_issue_content: "new issue".to_string(),
        };
        let event = NostrEvent::new_git_event(key_pair, git_event);

        let client_req = ClientMessage::new_event(event);
        let relay_peer_id = client_paras.rendezvous_point.unwrap();
        swarm
            .behaviour_mut()
            .nostr
            .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
    }

    pub async fn kad_get(&self, key: &str) {
        let key = kad::RecordKey::new(&key);
        {
            let mut swarm = self.swarm.lock().await;
            swarm.behaviour_mut().kademlia.get_record(key);
        }
    }

    pub async fn kad_put(&self, key: &str, value: &str) {
        let key = kad::RecordKey::new(&key);
        let value = value.as_bytes().to_vec();
        let record = Record {
            key,
            value,
            publisher: None,
            expires: None,
        };
        {
            let mut swarm = self.swarm.lock().await;
            if let Err(e) = swarm
                .behaviour_mut()
                .kademlia
                .put_record(record, Quorum::One)
            {
                eprintln!("Put record failed :{}", e);
            }
        }
    }

    pub async fn k_buckets(&self) {
        {
            let mut swarm = self.swarm.lock().await;
            for (_, k_bucket_ref) in swarm.behaviour_mut().kademlia.kbuckets().enumerate() {
                println!("k_bucket_ref.num_entries:{}", k_bucket_ref.num_entries());
                for (_, x) in k_bucket_ref.iter().enumerate() {
                    println!(
                        "PEERS[{:?}]={:?}",
                        x.node.key.preimage().to_string(),
                        x.node.value
                    );
                }
            }
        }
    }

    pub async fn get_peer(&self, peer_id: Option<&str>) {
        let peer_id = match parse_peer_id(peer_id) {
            Some(peer_id) => peer_id,
            None => {
                return;
            }
        };
        {
            let mut swarm = self.swarm.lock().await;
            swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
        }
    }
}

pub fn parse_peer_id(peer_id_str: Option<&str>) -> Option<PeerId> {
    match peer_id_str {
        Some(peer_id) => match PeerId::from_str(peer_id) {
            Ok(id) => Some(id),
            Err(err) => {
                eprintln!("peer_id parse error:{}", err);
                None
            }
        },
        None => {
            eprintln!("Expected peer_id");
            None
        }
    }
}

pub fn parse_mega_address(mega_address: &str) -> Result<(PeerId, &str), String> {
    // p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git
    let v: Vec<&str> = mega_address.split('/').collect();
    if v.len() < 4 {
        return Err("mega_address invalid".to_string());
    };
    let peer_id = match PeerId::from_str(v[2]) {
        Ok(peer_id) => peer_id,
        Err(e) => return Err(e.to_string()),
    };
    Ok((peer_id, v[3]))
}
