// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cmp::min;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::fs;

use std::rc::Rc;
use std::cell::RefCell;

use slint::{ComponentHandle, Image};

slint::include_modules!();
// const DELETE_FOLDER: String = "~/Pictures/deleted".to_string();

/// total number of images loaded in buffer
const BUFFER_SIZE :usize = 6;

/// Minimum number of elements to carry on either side of the buffer
const MIN_ELEM_NUM:usize = (BUFFER_SIZE-2)/2;



struct AppLogic {
    pic_list: Vec<PathBuf>,
    counter: usize,

    pic_buffer: Vec<Image>,
    name_buffer: Vec<String>,
    buffer_size: usize,
    buffer_idx: Vec<usize>,
    buffer_num: usize,
    front_file:usize,
    back_file:usize,

    edit_folder: PathBuf,
    delete_folder:PathBuf,
    current_folder:PathBuf
}

fn substract(a:usize, b:usize)-> usize{
    (((a as i32) - (b as i32) + BUFFER_SIZE as i32 )as usize)% BUFFER_SIZE
}

impl AppLogic {
    
    fn new(folder_path:PathBuf) -> Self {
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
        // let total_files = pic_list.len();
        pic_list.sort();
        let counter = 0;
        if pic_list.len() == 0 {
            panic!("Folder was empty")
        }
        let buffer_size= min(BUFFER_SIZE, pic_list.len());
        let mut pic_buffer: Vec<Image> = Vec::new();
        let mut name_buffer: Vec<String> = Vec::new();
        for i in 0..buffer_size {
            pic_buffer.push(Image::load_from_path(pic_list[i].as_path()).expect("image read failed"));
            name_buffer.push(pic_list[i].file_name().unwrap().to_str().unwrap().to_string());
        }
        let buffer_idx: Vec<usize> = (0..buffer_size).collect();
        let mut edit_folder = folder_path.clone();
        edit_folder.push("edit");
        let _ = fs::create_dir_all(edit_folder.clone());
        let mut delete_folder = folder_path.clone();
        delete_folder.push("bin");
        let _ = fs::create_dir_all(delete_folder.clone());

        // println!("{:?}",pic_list);
        Self{
            pic_list, 
            counter,

            pic_buffer,
            name_buffer,
            buffer_size,
            buffer_idx,
            buffer_num: 0,
            front_file: buffer_size-1,
            back_file: 0,

            edit_folder,
            delete_folder,
            current_folder: folder_path
        }
    }

    
    fn get_buffer_idx(&self, num:usize)-> usize{
        self.buffer_idx[num % self.buffer_size]
    }

    fn load_img_front(&mut self){
        let pic = self.pic_list[self.counter+self.front_file].clone();
        let buf_pos = self.get_buffer_idx(self.buffer_num+self.front_file);
        self.load_img(pic, buf_pos);
    }

    fn load_img_back(&mut self){
        let pic = self.pic_list[self.counter-self.back_file].clone();
        let buf_pos = self.get_buffer_idx(self.buffer_num + self.buffer_size - self.back_file);
        self.load_img(pic, buf_pos);
    }

    fn load_img(&mut self, pic:PathBuf, buf_pos:usize) {
        // TODO make async
        self.pic_buffer[buf_pos] = Image::load_from_path(pic.as_path()).expect("image read failed");
        self.name_buffer[buf_pos] = pic.file_name().unwrap().to_str().unwrap().to_string()
    }

    fn next_img(&mut self) -> bool {
        if self.counter == self.pic_list.len() -1 {
            return false;
        }
        self.buffer_num =(self.buffer_num+1)%self.buffer_size;
        self.counter += 1;
        if self.front_file > MIN_ELEM_NUM {
            self.front_file -= 1;
            self.back_file +=1;
            return true;
        }
        if self.counter+MIN_ELEM_NUM >= self.pic_list.len() {
            self.front_file -= 1;
            self.back_file +=1;
        }
        else {
            self.load_img_front();
        }
        true
    }

    fn prev_img(&mut self) -> bool {
        if self.counter == 0 {
            return false;
        }

        self.buffer_num = (self.buffer_num+self.buffer_size-1)%self.buffer_size;
        self.counter -= 1;
        if self.back_file > MIN_ELEM_NUM {
            self.back_file -= 1;
            self.front_file +=1;
            return true;
        }
        if self.counter < self.back_file  {
            self.back_file -= 1;
            self.front_file +=1;
        }
        else {
            self.load_img_back();
        }
        true
    }

    fn edit(&self) {
        let (file1, file2, dest1, dest2) = self.get_current_move_path(self.edit_folder.clone());

        fs::copy(file1, &dest1).unwrap();

        if fs::copy(file2, &dest2).is_err() {
            println!("No RAW file, only jpg was copied to edit")
        }
        else {
            println!("Copied to edit successfully")
        }
    }

    fn delete(&mut self) -> bool{
        let (file1, file2, dest1, dest2) = self.get_current_move_path(self.delete_folder.clone());
        fs::rename(file1, &dest1).unwrap();
        if fs::rename(file2, &dest2).is_err() {
            println!("No RAW file, only jpg was moved to bin")
        }
        else {
            println!("Moved to bin successfully")
        }
        self.pic_list.remove(self.counter);
        let buf_num = self.get_buffer_idx(self.buffer_num);
        if self.pic_list.len() == 0 {
            println!("No more photos, everything in the folder was deleted");
            return false;
        }
        if self.pic_list.len() < BUFFER_SIZE {
            self.buffer_idx.retain(|value| *value != buf_num);
            self.buffer_size -=1;
            if self.front_file == 0 {
                self.counter -= 1;
                self.back_file -=1;
                self.buffer_num -=1;
                return true;
            }
            self.front_file -=1;
            return true;
        }
        if self.front_file == 0 {
            for i in 0..self.back_file {
                let buf_num1 = self.get_buffer_idx(self.buffer_num+self.buffer_size-i);
                let buf_num2 = self.get_buffer_idx(self.buffer_num+self.buffer_size-i-1);
                self.pic_buffer[buf_num1] = self.pic_buffer[buf_num2].clone();
                self.name_buffer[buf_num1] = self.name_buffer[buf_num2].clone();
            }
            self.counter -= 1;
            self.load_img_back();
            return true;
        }
        for i in 0..self.front_file {
            let buf_num1 = self.get_buffer_idx(self.buffer_num+i);
            let buf_num2 = self.get_buffer_idx(self.buffer_num+i+1);
            self.pic_buffer[buf_num1] = self.pic_buffer[buf_num2].clone();
            self.name_buffer[buf_num1] = self.name_buffer[buf_num2].clone();
        }

        if self.counter+MIN_ELEM_NUM >= self.pic_list.len() {
            self.front_file -= 1;
            self.back_file +=1;
            self.load_img_back();
        }
        else {
            self.load_img_front();
        }
        true
    }

    fn get_img(&self) -> Image {
        let mut before: Vec<String> = Vec::new();
        let mut after: Vec<String> = Vec::new();
        for i in 0..self.back_file {
            let buf_num = self.get_buffer_idx(self.buffer_num + self.buffer_size+  i - self.back_file);
            before.push(self.name_buffer[buf_num].clone())
        }
        for i in 0..self.front_file {
            let buf_num = self.get_buffer_idx(self.buffer_num + i+1);
            after.push(self.name_buffer[buf_num].clone())
        }
        // println!("{:?}, {} {:?} (real: {} | buf_num: {})", before, self.name_buffer[self.get_buffer_idx(self.buffer_num)], after, self.counter, self.get_buffer_idx(self.buffer_num));
        // println!("{} real: {}", self.name_buffer[self.buffer_num], self.counter);
        self.pic_buffer[self.get_buffer_idx(self.buffer_num)].clone()
    }

    fn get_img_infos(&self)-> (String, usize, usize){
        (self.name_buffer[self.get_buffer_idx(self.buffer_num)].clone(), self.counter+1, self.pic_list.len())
    }

    fn get_current_move_path(&self, folder_move: PathBuf) -> (PathBuf,PathBuf,PathBuf,PathBuf){
        let mut file1 = self.current_folder.clone();
        file1.push(self.name_buffer[self.get_buffer_idx(self.buffer_num)].clone());
        let mut file2 = file1.clone();
        file2.set_extension("RAW");
    
        let mut dest1 = folder_move.clone();
        dest1.push(file1.file_name().unwrap());
        let mut dest2 = folder_move;
        dest2.push(file2.file_name().unwrap());
        (file1, file2, dest1, dest2) 
    }
}


fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    ui.window().set_maximized(true);
    // TODO PARAM
    let folder_path = PathBuf::from_str("/home/guilhem/Pictures/TEST").unwrap();
    // let folder_path = PathBuf::from_str("/home/guilhem/Documents/SAVE/Photos/2024_20 - Palawan").unwrap();
    // let mut logic = AppLogic::new(folder_path);
    let logic = Rc::new(RefCell::new(AppLogic::new(folder_path))); // Wrap in Rc<RefCell<>>
    ui.set_photo_path(logic.borrow().get_img());
    let (photo_name, photo_num, total_num) = logic.borrow().get_img_infos();
    ui.set_total_num(total_num as i32);
    ui.set_photo_name(photo_name.into());
    ui.set_photo_num(photo_num as i32);
    ui.on_next({
        let ui_handle = ui.as_weak();
        let l = logic.clone();
        move || {
            let mut logic_c = l.borrow_mut(); // Clone the Rc
            let ui = ui_handle.unwrap();
                if logic_c.next_img() {
                    ui.set_photo_path(logic_c.get_img());
                    let (photo_name, photo_num , _total_num) = logic_c.get_img_infos();
                    ui.set_photo_num(photo_num as i32);
                    ui.set_photo_name(photo_name.into());
                }
            }
        
    });
    ui.on_prev({
        let ui_handle = ui.as_weak();
        let l = logic.clone();

        move || {
            let mut logic_c = l.borrow_mut(); // Clone the Rc
            let ui = ui_handle.unwrap();
            if logic_c.prev_img() {
                ui.set_photo_path(logic_c.get_img());
                let (photo_name, photo_num , _total_num) = logic_c.get_img_infos();
                ui.set_photo_num(photo_num as i32);
                ui.set_photo_name(photo_name.into());
            }
        }
    });

    ui.on_edit({
        let l = logic.clone();
        move || {
            let logic_c = l.borrow(); // Clone the Rc
            logic_c.edit();
        }
    });

    ui.on_delete({
        let ui_handle = ui.as_weak();
        let l = logic.clone();

        move || {
            let mut logic_c = l.borrow_mut(); // Clone the Rc
            let ui = ui_handle.unwrap();
            if logic_c.delete(){
                ui.set_photo_path(logic_c.get_img());
                let (photo_name, photo_num , total_num) = logic_c.get_img_infos();
                ui.set_photo_num(photo_num as i32);
                ui.set_total_num(total_num as i32);
                ui.set_photo_name(photo_name.into());
            }
            else {
                let _  = slint::quit_event_loop();
            }
        }
    });
    ui.on_close(|| {let _  = slint::quit_event_loop();});


    ui.run()?;

    Ok(())
}
