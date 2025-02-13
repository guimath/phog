// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::fs;

use std::rc::Rc;
use std::cell::RefCell;

use slint::Image;

slint::include_modules!();
// const DELETE_FOLDER: String = "~/Pictures/deleted".to_string();
const BUFFER_SIZE :usize = 6;
const BUFFER_OFFSET:usize = BUFFER_SIZE/2;
struct AppLogic {
    pic_list: Vec<PathBuf>,
    counter: usize,
    pic_buffer: [Image; BUFFER_SIZE],
    buffer_num: usize,
    front_file:usize,
    back_file:usize,
    dbg: [String; BUFFER_SIZE],
}

fn substract(a:usize, b:usize)-> usize{
    (((a as i32) - (b as i32) + BUFFER_SIZE as i32 )as usize)% BUFFER_SIZE
    
}

impl AppLogic {
    fn new(folder_path:PathBuf) -> Self {
        let files = fs::read_dir(folder_path).expect("Folder scan failed");
        let mut pic_list: Vec<PathBuf> = Vec::new();
        
        for file in files {
            let f_path = file.unwrap().path();
            if f_path.extension().is_none() { break;}
            let ext = f_path.extension().unwrap().to_str().unwrap();
            if ext == "JPG" {
                pic_list.push(f_path);
            }
        }
        // let total_files = pic_list.len();
        pic_list.sort();
        let counter = 0;
        
        let pic_buffer: [Image; BUFFER_SIZE] = (0..BUFFER_SIZE).map(|i| 
            Image::load_from_path(pic_list[i].as_path()).expect("image read failed")
        ).collect::<Vec<_>>()
        .try_into()
        .unwrap();
        let dbg: [String; BUFFER_SIZE] = (0..BUFFER_SIZE).map(|i|
            pic_list[i].file_stem().unwrap().to_str().unwrap().to_string()
        ).collect::<Vec<_>>()
        .try_into()
        .unwrap();
        // println!("{:?}",pic_list);
        Self{
            pic_list, 
            counter,
            pic_buffer,
            buffer_num: 0,
            front_file: BUFFER_SIZE-1,
            back_file: 0,
            dbg,
        }
    }

    fn next_img(&mut self) -> bool {
        if self.counter == self.pic_list.len() -1 {
            return false;
        }
        self.buffer_num =(self.buffer_num+1)%BUFFER_SIZE;
        self.counter += 1;
        if self.front_file > BUFFER_OFFSET {
            self.front_file -= 1;
            self.back_file +=1;
        }
        else {
            if self.counter+BUFFER_OFFSET >= self.pic_list.len() {
                self.front_file -= 1;
                self.back_file +=1;
            }
            else {
                self.pic_buffer[(self.buffer_num+BUFFER_OFFSET)%BUFFER_SIZE] = Image::load_from_path(self.pic_list[self.counter+BUFFER_OFFSET].as_path()).expect("image read failed");
                self.dbg[(self.buffer_num+BUFFER_OFFSET)%BUFFER_SIZE] = self.pic_list[self.counter+BUFFER_OFFSET].file_stem().unwrap().to_str().unwrap().to_string()
                
            }
        }
        
        true
    }

    fn prev_img(&mut self) -> bool {
        if self.counter == 0 {
            return false;
        }

        self.buffer_num =(self.buffer_num+BUFFER_SIZE-1)%BUFFER_SIZE;
        self.counter -= 1;
        if self.back_file > BUFFER_OFFSET {
            self.back_file -= 1;
            self.front_file +=1;
        }

        else {
            if self.counter < self.back_file  {
                self.back_file -= 1;
                self.front_file +=1;
            }
            else {
                self.pic_buffer[substract(self.buffer_num, self.back_file)] = Image::load_from_path(self.pic_list[self.counter-self.back_file].as_path()).expect("image read failed");
                self.dbg[substract(self.buffer_num, self.back_file)] = self.pic_list[self.counter-self.back_file].file_stem().unwrap().to_str().unwrap().to_string();
            }
        }
        // else {
        //     self.back_file += 1;
        //     self.front_file -=1;
        // }
        true
    }

    fn get_img(&mut self) -> Image {
        let mut before: Vec<String> = Vec::new();
        let mut after: Vec<String> = Vec::new();
        for i in 0..self.back_file {
            before.push(self.dbg[(self.buffer_num + BUFFER_SIZE+  i - self.back_file)%BUFFER_SIZE].clone())
        }
        for i in 0..self.front_file {
            after.push(self.dbg[(self.buffer_num+ (i+1))%BUFFER_SIZE].clone())
        }
        // println!("{} real: {}", self.dbg[self.buffer_num], self.counter);
        // println!("{:?}, {} {:?} (real: {} | buf_num: {})", before, self.dbg[self.buffer_num], after, self.counter, self.buffer_num);
        self.pic_buffer[self.buffer_num].clone()
    }
}


fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    // TODO PARAM
    // let folder_path = PathBuf::from_str("/home/guilhem/Pictures/TEST").unwrap();
    let folder_path = PathBuf::from_str("/home/guilhem/Documents/SAVE/Photos/2024_20 - Palawan").unwrap();
    // let mut logic = AppLogic::new(folder_path);
    let logic = Rc::new(RefCell::new(AppLogic::new(folder_path))); // Wrap in Rc<RefCell<>>
    ui.set_photo_path(logic.borrow_mut().get_img());
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        let l = logic.clone();
        move || {
            let mut logic_c = l.borrow_mut(); // Clone the Rc
            let ui = ui_handle.unwrap();
                if logic_c.next_img() {
                    ui.set_photo_path(logic_c.get_img());
                }
            }
        
    });
    ui.on_request_decrease_value({
        let ui_handle = ui.as_weak();
        let l = logic.clone();

        move || {
            let mut logic_c = l.borrow_mut(); // Clone the Rc
            let ui = ui_handle.unwrap();
            if logic_c.prev_img() {
                ui.set_photo_path(logic_c.get_img());
            }
        }
    });
    ui.run()?;

    Ok(())
}
