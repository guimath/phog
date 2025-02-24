use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use pgt::logic::AppLogic;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    ui.window().set_maximized(true);
    // TODO PARAM
    let folder_path = PathBuf::from_str("/home/guilhem/Pictures/TEST").unwrap();
    // let folder_path = PathBuf::from_str("/home/guilhem/Documents/SAVE/Photos/2024_20 - Palawan").unwrap();
    // let mut logic = AppLogic::new(folder_path);
    let logic = Arc::new(Mutex::new(AppLogic::new(folder_path))); // Wrap in Rc<RefCell<>>
    ui.set_photo_path(logic.lock().unwrap().get_img().await);
    let (photo_name, photo_num, total_num) = logic.lock().unwrap().get_img_infos();
    ui.set_total_num(total_num as i32);
    ui.set_photo_name(photo_name.into());
    ui.set_photo_num(photo_num as i32);
    ui.on_next({
        let ui_handle = ui.as_weak();
        let logic_ref = logic.clone();
        move || {
            let logic_c = logic_ref.clone();
            let ui = ui_handle.unwrap();
            let new_disp = logic_c.lock().unwrap().next_img();
            if new_disp {
                slint::spawn_local(async move {
                    let mut locked_logic = logic_c.lock().unwrap();
                    let img = locked_logic.get_img().await;
                    ui.set_photo_path(img);
                    let (photo_name, photo_num , _total_num) = locked_logic.get_img_infos();
                    ui.set_photo_num(photo_num as i32);
                    ui.set_photo_name(photo_name.into());
                }).unwrap();
            }
            }
        
        
    });
    ui.on_prev({
        let ui_handle = ui.as_weak();
        let logic_ref = logic.clone();
        move || {
            let logic_c = logic_ref.clone();
            let ui = ui_handle.unwrap();
            slint::spawn_local(async move {
                let mut locked_logic = logic_c.lock().unwrap();
                if locked_logic.prev_img() {

                    ui.set_photo_path(locked_logic.get_img().await);
                    let (photo_name, photo_num , _total_num) = locked_logic.get_img_infos();
                    ui.set_photo_num(photo_num as i32);
                    ui.set_photo_name(photo_name.into());
                }
            }).unwrap();
        }
    });

    ui.on_edit({
        let logic_ref = logic.clone();
        move || {
            let logic_c = logic_ref.clone(); // Clone the Rc
            logic_c.lock().unwrap().edit();
        }
    });

    ui.on_delete({
        let ui_handle = ui.as_weak();
        let logic_ref = logic.clone();
        move || {
            let logic_c = logic_ref.clone();
            let ui = ui_handle.unwrap();
            let new_disp = logic_c.lock().unwrap().delete();
            if new_disp{
                slint::spawn_local(async move {
                    let mut locked_logic = logic_c.lock().unwrap();
                    ui.set_photo_path(locked_logic.get_img().await);
                    let (photo_name, photo_num , total_num) = locked_logic.get_img_infos();
                    ui.set_photo_num(photo_num as i32);
                    ui.set_total_num(total_num as i32);
                    ui.set_photo_name(photo_name.into());
                }).unwrap();
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
