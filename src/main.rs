use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use pgt::logic::{AppLogic, ImageStat};

slint::include_modules!();

const MIN_DELAY: Duration = Duration::from_millis(100);

/// Syntactic sugar for async in slint callback
/// 
/// ### Params 
/// 1) existing App Window variable
/// 2) name of ui handle to be used for ui calls inside async
/// 3) existing Mutex App Logic variable 
/// 4) name of the locked logic name to be used for logic calls inside async 
/// 5) all the code to be placed in async block (isolated by brackets)
macro_rules! async_context {
    ($last_date:ident, $ui:ident, $logic:ident, $code:block) => {{
        let ui_handle:slint::Weak<AppWindow> = $ui.as_weak();
        let logic_ref: Arc<Mutex<AppLogic>> = $logic.clone();
        move || {
            if Instant::now()- $last_date < MIN_DELAY {
                return 
            }
            $last_date = Instant::now();
            let logic_c = logic_ref.clone();
            #[allow(unused)]
            let $ui = ui_handle.unwrap();
            slint::spawn_local(async_compat::Compat::new(async move { 
                #[allow(unused_mut)]
                let mut $logic = logic_c.lock().await;
                $code 
            })).unwrap();
        }
    }}
}


// no #[tokio::main] because it crashes after a few Mutex locks (compatibility issue with slint)
fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    ui.window().set_maximized(true);
    // TODO PARAM
    // let folder_path = PathBuf::from_str("/home/guilhem/Pictures/TEST").unwrap();
    let folder_path = PathBuf::from_str("/home/guilhem/Documents/SAVE/Photos/2024_20 - Palawan").unwrap();
    let logic = Arc::new(Mutex::new(AppLogic::new(folder_path))); 
    let first: ImageStat = logic.blocking_lock().get_first_img();
    ui.set_total_num(first.out_of as i32);
    ui.set_photo_num(first.number as i32);
    ui.set_photo_name(first.name.into());
    ui.set_photo_path(first.image);
    let mut last_cmd = Instant::now();
    
    let logic_c = logic.clone();
    slint::spawn_local(async_compat::Compat::new(async move {
        let mut logic = logic_c.lock().await;
        logic.init().await;
    })).unwrap();

   
    ui.on_next(async_context!{last_cmd, ui, logic, {
        if logic.next_img().await{
            let img: ImageStat = logic.get_img().await;
            ui.set_photo_path(img.image);
            ui.set_photo_num(img.number as i32);
            ui.set_photo_name(img.name.into());
        }
    }});

    ui.on_prev(async_context!{last_cmd, ui, logic, {
        if logic.prev_img().await {
            let img: ImageStat = logic.get_img().await;
            ui.set_photo_path(img.image);
            ui.set_photo_num(img.number as i32);
            ui.set_photo_name(img.name.into());
        }
    }});


    ui.on_edit(async_context!{last_cmd, ui, logic, {
        logic.edit();
    }}); 
    
    ui.on_delete(async_context!{last_cmd, ui, logic, {
        if logic.delete().await {
            let img: ImageStat = logic.get_img().await;
            ui.set_photo_path(img.image);
            ui.set_photo_num(img.number as i32);
            ui.set_total_num(img.out_of as i32);
            ui.set_photo_name(img.name.into());
        }
        else {
            slint::quit_event_loop().unwrap();
        }
    }});

    ui.on_close(|| {slint::quit_event_loop().unwrap();});

    ui.run()?;

    Ok(())
}
