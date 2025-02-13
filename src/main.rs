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

/// total number of images loaded in buffer
const BUFFER_SIZE :usize = 6;

/// Minimum number of elements to carry on either side of the buffer
const MIN_ELEM_NUM:usize = (BUFFER_SIZE-2)/2;

struct AppLogic {
    pic_list: Vec<PathBuf>,
    counter: usize,
    pic_buffer: [Image; BUFFER_SIZE],
    name_buffer: [String; BUFFER_SIZE],
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
        
        let pic_buffer: [Image; BUFFER_SIZE] = (0..BUFFER_SIZE).map(|i| 
            Image::load_from_path(pic_list[i].as_path()).expect("image read failed")
        ).collect::<Vec<_>>()
        .try_into()
        .unwrap();
        let name_buffer: [String; BUFFER_SIZE] = (0..BUFFER_SIZE).map(|i|
            pic_list[i].file_name().unwrap().to_str().unwrap().to_string()
        ).collect::<Vec<_>>()
        .try_into()
        .unwrap();

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
            buffer_num: 0,
            front_file: BUFFER_SIZE-1,
            back_file: 0,
            name_buffer,
            edit_folder,
            delete_folder,
            current_folder: folder_path
        }
    }

    fn next_img(&mut self) -> bool {
        if self.counter == self.pic_list.len() -1 {
            return false;
        }
        self.buffer_num =(self.buffer_num+1)%BUFFER_SIZE;
        self.counter += 1;
        if self.front_file > MIN_ELEM_NUM {
            self.front_file -= 1;
            self.back_file +=1;
        }
        else {
            if self.counter+MIN_ELEM_NUM >= self.pic_list.len() {
                self.front_file -= 1;
                self.back_file +=1;
            }
            else {
                self.pic_buffer[(self.buffer_num+MIN_ELEM_NUM)%BUFFER_SIZE] = Image::load_from_path(self.pic_list[self.counter+MIN_ELEM_NUM].as_path()).expect("image read failed");
                self.name_buffer[(self.buffer_num+MIN_ELEM_NUM)%BUFFER_SIZE] = self.pic_list[self.counter+MIN_ELEM_NUM].file_name().unwrap().to_str().unwrap().to_string()
                
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
        if self.back_file > MIN_ELEM_NUM {
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
                self.name_buffer[substract(self.buffer_num, self.back_file)] = self.pic_list[self.counter-self.back_file].file_name().unwrap().to_str().unwrap().to_string();
            }
        }
        // else {
        //     self.back_file += 1;
        //     self.front_file -=1;
        // }
        true
    }

    fn edit(&self) {
        let mut file1 = self.current_folder.clone();
        file1.push(self.name_buffer[self.buffer_num].clone());
        let mut file2 = file1.clone();
        file2.set_extension("RAW");

        let mut dest1 = self.edit_folder.clone();
        dest1.push(file1.file_name().unwrap());
        let mut dest2 = self.edit_folder.clone();
        dest2.push(file2.file_name().unwrap());

        fs::copy(file1, &dest1).unwrap();

        if fs::copy(file2, &dest2).is_err() {
            println!("No RAW file, only jpg was copied")
        }
    }

    fn get_img(&self) -> Image {
        // let mut before: Vec<String> = Vec::new();
        // let mut after: Vec<String> = Vec::new();
        // for i in 0..self.back_file {
        //     before.push(self.name_buffer[(self.buffer_num + BUFFER_SIZE+  i - self.back_file)%BUFFER_SIZE].clone())
        // }
        // for i in 0..self.front_file {
        //     after.push(self.name_buffer[(self.buffer_num+ (i+1))%BUFFER_SIZE].clone())
        // }
        println!("{} real: {}", self.name_buffer[self.buffer_num], self.counter);
        // println!("{:?}, {} {:?} (real: {} | buf_num: {})", before, self.name_buffer[self.buffer_num], after, self.counter, self.buffer_num);
        self.pic_buffer[self.buffer_num].clone()
    }

    fn get_img_infos(&self)-> (String, usize, usize){
        (self.name_buffer[self.buffer_num].clone(), self.counter+1, self.pic_list.len())
    }
}


fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

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

    ui.run()?;

    Ok(())
}
