
use std::path::PathBuf;
use crate::circ_buf::CircularBuffer;
pub use crate::circ_buf::ImageStat;
use std::fs;

pub struct AppLogic {
    buffer: CircularBuffer,
    edit_folder: PathBuf,
    delete_folder:PathBuf,
    current_folder:PathBuf,
    current_name:String,
}

impl AppLogic {
    
    pub fn new(folder_path:PathBuf) -> Self {
        let files = fs::read_dir(folder_path.clone()).expect("Folder scan failed");
        let mut pic_list: Vec<PathBuf> = Vec::new();
        
        for file in files {
            let f_path = file.unwrap().path();
            if f_path.extension().is_none() { continue;}
            let ext = f_path.extension().unwrap().to_str().unwrap();
            if ext == "JPG" {
                pic_list.push(f_path);
            }
        }
        pic_list.sort();
        if pic_list.len() == 0 {
            panic!("Folder was empty")
        }
        let buffer = CircularBuffer::new(pic_list);
        let mut edit_folder = folder_path.clone();
        edit_folder.push("edit");
        let mut delete_folder = folder_path.clone();
        delete_folder.push("bin");
        Self{
            buffer, 
            edit_folder,
            delete_folder,
            current_folder: folder_path,
            current_name: String::new()
        }
    }


    pub async fn next_img(&mut self) -> bool {
        self.buffer.next_img().await
    }

    pub async fn prev_img(&mut self) -> bool {
        self.buffer.prev_img().await
    }

    pub fn edit(&self) {
        // TODO EDIT detect already selected 

        let _ = fs::create_dir_all(self.edit_folder.clone());
        let (file1, file2, dest1, dest2) = self.get_current_move_path(self.edit_folder.clone());
        fs::copy(file1, &dest1).unwrap();
        if fs::copy(file2, &dest2).is_err() {
            println!("No RAW file, only jpg was copied to edit")
        }
        else {
            println!("Copied to edit successfully")
        }
    }

    pub async fn delete(&mut self) -> bool{
        let _ = fs::create_dir_all(self.delete_folder.clone());
        let (file1, file2, dest1, dest2) = self.get_current_move_path(self.delete_folder.clone());
        fs::rename(file1, &dest1).unwrap();
        if fs::rename(file2, &dest2).is_err() {
            println!("No RAW file, only jpg was moved to bin")
        }
        else {
            println!("Moved to bin successfully")
        }
        self.buffer.delete().await
    }
    pub async fn init(&mut self) {
        self.buffer.init().await;
    }

    pub async fn get_img(&mut self) -> ImageStat {
        let img = self.buffer.get_elem().await;
        self.current_name = img.name.clone();
        img
    }

    pub fn get_first_img(&mut self) -> ImageStat {
        let img = self.buffer.get_first_elem();
        self.current_name = img.name.clone();
        img
    }

    fn get_current_move_path(&self, folder_move: PathBuf) -> (PathBuf,PathBuf,PathBuf,PathBuf){
        let mut file1 = self.current_folder.clone();
        file1.push(self.current_name.clone());
        let mut file2 = file1.clone();
        file2.set_extension("RAF");
    
        let mut dest1 = folder_move.clone();
        dest1.push(file1.file_name().unwrap());
        let mut dest2 = folder_move;
        dest2.push(file2.file_name().unwrap());
        (file1, file2, dest1, dest2) 
    }
}
