pub mod entity;
//pub use entity::*;

pub mod utils;
//pub use utils::*;

 //

pub mod rev_anno;
// Uncommented -> a similar path error
//pub use rev_anno::*; // keep all 'use' like this commented

 

pub mod mda_operations{
    pub mod generate;
    pub mod extract;
    pub mod update;
}
// pub use mda_operations::generate;
// pub use mda_operations::extract;
// pub use mda_operations::update;

pub mod map{
    pub mod read_from_file;
    pub mod read_from_folders;
}
//pub use map::*;


pub mod run_mda;