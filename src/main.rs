use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use pgt::logic::AppLogic;

slint::include_modules!();


/// Syntactic sugar for async in slint callback
/// 
/// ### Params 
/// 1) existing App Window variable
/// 2) name of ui handle to be used for ui calls inside async
/// 3) existing Mutex App Logic variable 
/// 4) name of the locked logic name to be used for logic calls inside async 
/// 5) all the code to be placed in async block (isolated by brackets)
macro_rules! async_context {
    ($ui:ident, $ui_clone:ident, $logic_arc:ident, $logic_locked:ident, $code:block) => {{
        let ui_handle = $ui.as_weak();
        let logic_ref = $logic_arc.clone();
        move || {
            let logic_c = logic_ref.clone();
            let $ui_clone = ui_handle.unwrap();
            slint::spawn_local(async move { 
                let mut $logic_locked = logic_c.lock().unwrap();
                $code 
            }).unwrap();
        }
    }}
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    ui.window().set_maximized(true);
    // TODO PARAM
    // let folder_path = PathBuf::from_str("/home/guilhem/Pictures/TEST").unwrap();
    let folder_path = PathBuf::from_str("/home/guilhem/Documents/SAVE/Photos/2024_20 - Palawan").unwrap();
    let logic = Arc::new(Mutex::new(AppLogic::new(folder_path))); // Wrap in Rc<RefCell<>>
    // TODO patch first photo
    ui.set_photo_path(logic.lock().unwrap().get_img().await);
    let (photo_name, photo_num, total_num) = logic.lock().unwrap().get_img_infos();
    ui.set_total_num(total_num as i32);
    ui.set_photo_name(photo_name.into());
    ui.set_photo_num(photo_num as i32);

    ui.on_next(async_context!{ui, ui_c, logic, locked_logic, {
        if locked_logic.next_img().await{
            ui_c.set_photo_path(locked_logic.get_img().await);
            let (photo_name, photo_num , _total_num) = locked_logic.get_img_infos();
            ui_c.set_photo_num(photo_num as i32);
            ui_c.set_photo_name(photo_name.into());
        }
    }});

    ui.on_prev(async_context!{ui, ui_c, logic, locked_logic, {
        if locked_logic.prev_img().await {
            ui_c.set_photo_path(locked_logic.get_img().await);
            let (photo_name, photo_num , _total_num) = locked_logic.get_img_infos();
            ui_c.set_photo_num(photo_num as i32);
            ui_c.set_photo_name(photo_name.into());
        }
    }});


    ui.on_edit({
        let logic_ref = logic.clone();
        move || {
            let logic_c = logic_ref.clone(); // Clone the Rc
            logic_c.lock().unwrap().edit();
        }
    });

    ui.on_delete(async_context!{ui, ui_c, logic, locked_logic, {
        if locked_logic.delete().await {
            ui_c.set_photo_path(locked_logic.get_img().await);
            let (photo_name, photo_num , total_num) = locked_logic.get_img_infos();
            ui_c.set_photo_num(photo_num as i32);
            ui_c.set_total_num(total_num as i32);
            ui_c.set_photo_name(photo_name.into());
        }
        else {
            slint::quit_event_loop().unwrap();
        }
    }});

    ui.on_close(|| {slint::quit_event_loop().unwrap();});

    ui.run()?;

    Ok(())
}
