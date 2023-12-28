use std::env;
// 需要引入文件夹名字
mod utils;
// 需要写入utils里面在根目录下引入的文件
mod entity;
mod run_mda;
// map和mda_operations的同理
mod map;
mod mda_operations;
mod rev_anno;

// 引入和命令行语句操作相关的
use anyhow::Result;
use clap::Parser;
use mda::run_mda::run;
use std::error::Error;
use libp2p::{identity,PeerId};

// #[tokio::main]
// async fn main(){
//     // test p2p in mda
//     // 密钥对
//     let new_key = identity::Keypair::generate_ed25519();
//     // 使用密钥对的公钥生成peerID
//     let new_peerID = PeerId::from(new_key.public());
//     println!("New peer id is {:?}", new_peerID);
// }
fn main() {
    // 解析命令行参数
    let opts = mda::run_mda::MDAOptions::parse();
    // 根据参数运行程序
    if let Err(e) = run(opts){
        eprintln!("Application error:{}",e);
        std::process::exit(1); // 强制退出
    }
    
}
