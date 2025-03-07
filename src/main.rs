use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use phog::logic::{AppLogic, ImageStat, AppWindow};
use slint::ComponentHandle;
const MIN_DELAY: Duration = Duration::from_millis(600);

/// Syntactic sugar for async in slint callback
///
/// ### Params
/// 1) (optional) last date to be checked against min_delay if key was repeated 
/// 2) UI app
/// 3) Logic variable
/// 5) all the code to be placed in async block (isolated by brackets)
macro_rules! async_context {
    ($last_date:ident, $ui:ident, $logic:ident, $code:block) => {{
        let ui_handle = $ui.as_weak();
        let logic_ref: Arc<Mutex<AppLogic>> = $logic.clone();
        move |repeat: bool| {
            if repeat {
                if Instant::now() - $last_date < MIN_DELAY {
                    return;
                }
                $last_date = Instant::now();
            }
            let logic_c = logic_ref.clone();
            #[allow(unused)]
            let $ui = ui_handle.unwrap();
            slint::spawn_local(async_compat::Compat::new(async move {
                #[allow(unused_mut)]
                let mut $logic = logic_c.lock().await;
                $code
            }))
            .unwrap();
        }
    }};
    ($ui:ident, $logic:ident, $code:block) => {{ // same but not repeat param
        let ui_handle = $ui.as_weak();
        let logic_ref: Arc<Mutex<AppLogic>> = $logic.clone();
        move || {
            let logic_c = logic_ref.clone();
            #[allow(unused)]
            let $ui = ui_handle.unwrap();
            slint::spawn_local(async_compat::Compat::new(async move {
                #[allow(unused_mut)]
                let mut $logic = logic_c.lock().await;
                $code
            }))
            .unwrap();
        }
    }};
}

macro_rules! update_image {
    ($ui:ident, $logic:ident) => {{
        let img: ImageStat = $logic.get_img().await;
        update_image_only!($ui, img);
    }};
}

macro_rules! update_image_only {
    ($ui:ident, $img:ident) => {{
        $ui.set_photo_path($img.image);
        $ui.set_photo_num($img.number as i32);
        $ui.set_total_num($img.out_of as i32);
        $ui.set_photo_name($img.name.into());
    }}
}
// TODO PARAM ?

// no #[tokio::main] because it crashes after a few Mutex locks (compatibility issue with slint)
fn main() -> Result<(), Box<dyn Error>> {
    // Attempts to find locale translation (default English)
    slint::init_translations!(concat!(env!("CARGO_MANIFEST_DIR"), "/lang/"));
    let ui = AppWindow::new()?;
    ui.window().set_maximized(true);
    let folder_path = std::env::current_dir()?;
    let logic = Arc::new(Mutex::new(AppLogic::new(folder_path, "edit".into(), "bin".into())));
    let first: ImageStat = logic.blocking_lock().get_first_img();
    update_image_only!(ui, first);
    let mut last_cmd = Instant::now();

    let logic_c = logic.clone();
    slint::spawn_local(async_compat::Compat::new(async move {
        let mut logic = logic_c.lock().await;
        logic.init().await;
    }))
    .unwrap();

    ui.on_next(async_context! {last_cmd, ui, logic, {
        if logic.next_img().await{
            update_image!(ui, logic);
        }
    }});

    ui.on_prev(async_context! {last_cmd, ui, logic, {
        if logic.prev_img().await {
            update_image!(ui, logic);
        }
    }});
    ui.on_edit(async_context! {ui, logic, {
        ui.invoke_display_message(logic.edit());
    }});

    ui.on_delete(async_context! {ui, logic, {
        let (status, to_update) = logic.delete().await;
        ui.invoke_display_message(status);
        if to_update {
            update_image!(ui, logic);
        }
        else {
            slint::quit_event_loop().unwrap();
        }
    }});

    ui.on_prep_bin_input(async_context! {ui, logic, {
        ui.invoke_display_text_input(logic.get_delete_folder().into());
    }});
    ui.on_prep_edit_input(async_context! {ui, logic, {
        ui.invoke_display_text_input(logic.get_edit_folder().into());
    }});
    ui.on_set_bin_input(async_context! {ui, logic, {
        logic.set_delete_folder(ui.get_text_input().into());
    }});
    ui.on_set_edit_input(async_context! {ui, logic, {
        logic.set_edit_folder(ui.get_text_input().into());
    }});

    ui.on_close(|| {
        slint::quit_event_loop().unwrap();
    });

    ui.run()?;

    Ok(())
}
