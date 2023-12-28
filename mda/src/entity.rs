//！ Store some entity.

use serde::{Deserialize, Serialize};

 

#[derive(Serialize, Deserialize, Debug)]
pub struct MDAIndex {
    pub header_offset: u64,
    pub train_data_offset: u64,
    pub annotations_offset:Vec<AnnoOffset>
}
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct AnnoOffset{
    pub id:String,
    pub header_offset:u64,
    pub entries_offset:u64,
}
impl AnnoOffset {
    pub fn new (id:&str)->AnnoOffset{
        AnnoOffset { id: id.to_string(), header_offset: 0, entries_offset: 0 }
    }
}
/// Define the MDAHeader structure
#[derive(Serialize, Deserialize, Debug)]
pub struct MDAHeader {
    pub tags: Vec<String>,
    pub train_data: TrainData,
}

/// Define the train_data_index in header
#[derive(Serialize, Deserialize, Debug)]
pub struct TrainData {
    pub data_type: String,
    pub metadata: String,
}

/// Type of training data
#[derive(Serialize, Deserialize, Debug)]
pub enum TrainingData {
    Text(String),
    Image(Vec<u8>),
    Video(Vec<u8>),
    Audio(Vec<u8>),
}

/// Type of training data
#[derive(Serialize, Deserialize, Debug)]
pub enum DataType {
    Text,
    Image,
    Video,
    Audio,
}

/// Used to store the image metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct ImageMetaData {
    pub size: (u32, u32),
    pub channel_count: u8,
    pub color_space: String,
}

/// Used to store the text metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct TextMetaData {
    pub length: usize,
    pub encoding: String,
    pub vocabulary_size: usize,
}

/// Used to store the aduio metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct AudioMetaData {
    pub duration: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
}

// VideoMetaData
#[derive(Debug, Clone)]
pub struct VideoMetaData {
    pub duration: f64,
    pub resolution: (u16, u16),
}

// AIBOM Properties 
pub struct AIBOM{
    pub name: String,// Dataset name
    pub location: String,// Dataset official website
    pub license_location: String,// License address link
    pub content_type: String,// Dataset content type (this can be set as ENUM)
    pub size: u32,//Total size of the dataset,
    pub intended_use: String,//Official purpose of use of the dataset
    pub checksum: Vec<u8>,//checksum
    pub have_known_biases: bool,//whether there are any biases (What's bias for?)
    pub originator: Vec<String>,//list of contributors, separated by ',' and ' '
    pub data_collection_process: String,//as the name tells
    pub concluded_license: String,//License certified in SPDX License List
    pub declared_license: String,//Customize the content of the license (such as Chinese and English keywords such as license, term of use, citation and reference, etc.)
    pub is_sensitive_personal_information: bool,//whether there is personal privacy information
    pub rejection_notes: String,//if this dataset review is rejected, show the reason
    pub have_offensive_content:bool,//Whether there is offensive content
    pub topics: String//the field the dataset locates
}