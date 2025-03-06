
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

pub enum FileMoveStatus {
    Successfull,
    NoRAW,
    AlreadyDone,
    Failed,
}

impl AppLogic {
    
    pub fn new(folder_path:PathBuf, edit_folder_name:String, delete_folder_name:String) -> Self {
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
        edit_folder.push(edit_folder_name);
        let mut delete_folder = folder_path.clone();
        delete_folder.push(delete_folder_name);
        Self{
            buffer, 
            edit_folder,
            delete_folder,
            current_folder: folder_path,
            current_name: String::new()
        }
    }

    pub fn set_edit_folder(&mut self, name:String) {
        self.edit_folder.set_file_name(name);
    }
    pub fn get_edit_folder(&mut self) -> String {
        self.edit_folder.file_name().unwrap().to_str().unwrap().into()
    }
    pub fn set_delete_folder(&mut self, name:String) {
        self.delete_folder.set_file_name(name);
    }
    pub fn get_delete_folder(&mut self) -> String {
        self.delete_folder.file_name().unwrap().to_str().unwrap().into()
    }

    pub async fn next_img(&mut self) -> bool {
        self.buffer.next_img().await
    }

    pub async fn prev_img(&mut self) -> bool {
        self.buffer.prev_img().await
    }

    pub fn edit(&self) -> FileMoveStatus {
        let _ = fs::create_dir_all(self.edit_folder.clone());
        let (file1, file2, dest1, dest2) = self.get_current_move_path(self.edit_folder.clone());
        if fs::exists(dest1.clone()).unwrap(){
            return FileMoveStatus::AlreadyDone;
        }
        if fs::copy(file1, &dest1).is_err() {
            return FileMoveStatus::Failed;
        }
        if fs::copy(file2, &dest2).is_err() {
            return FileMoveStatus::NoRAW;
        }
        FileMoveStatus::Successfull
    }

    pub async fn delete(&mut self) -> (FileMoveStatus, bool){
        let _ = fs::create_dir_all(self.delete_folder.clone());
        let (file1, file2, dest1, dest2) = self.get_current_move_path(self.delete_folder.clone());
        if !fs::exists(file1.clone()).unwrap(){
            return (FileMoveStatus::AlreadyDone, true);
        }
        if fs::rename(file1, &dest1).is_err() {
            return (FileMoveStatus::Failed, true);
        }
        let status = if fs::rename(file2, &dest2).is_err() {
            FileMoveStatus::NoRAW
        } else {
            FileMoveStatus::Successfull
        };
        (status, self.buffer.delete().await)
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
