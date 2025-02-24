use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::rc::Rc;
use std::cell::RefCell;

use pgt::logic::AppLogic;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    ui.window().set_maximized(true);
    // TODO PARAM
    // let folder_path = PathBuf::from_str("/home/guilhem/Pictures/TEST").unwrap();
    let folder_path = PathBuf::from_str("/home/guilhem/Documents/SAVE/Photos/2024_20 - Palawan").unwrap();
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
