use std::cell::RefCell;
use std::cmp::min;
use std::fs::{self, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::process::{exit, Command};
use std::thread::sleep;
use std::time::{/*Instant, SystemTime,*/ Duration};
use std::{mem, env, str};

use chrono::Local;
use image::{ImageBuffer, Luma, DynamicImage, Pixel, /*FilterType,*/ load_from_memory};
// use image::imageops::{resize, overlay /*, brighten*/};
use libc::ioctl;
use memmap::{MmapOptions, MmapMut};
use fb::{FbVarScreenInfo, FbFixScreenInfo, MxcfbUpdateData51};

mod fb;

thread_local! {
    static LOCAL_MEME_ID: RefCell<Option<i32>> = RefCell::new(None);
}

fn get_vscreen_info(fb_desc: i32) -> Result<FbVarScreenInfo, String> {
    let mut vinfo: FbVarScreenInfo = unsafe { mem::zeroed() };
    let result = unsafe { ioctl(fb_desc, 0x4600, &mut vinfo) };
    if result != 0 {
        Err("error with ioctl FBIOGET_VSCREENINFO".to_string())
    } else {
        Ok(vinfo)
    }
}

fn get_fscreen_info(fb_desc: i32) -> Result<FbFixScreenInfo, String> {
    let mut finfo: FbFixScreenInfo = unsafe { mem::zeroed() };
    let result = unsafe { ioctl(fb_desc, 0x4602, &mut finfo) };
    if result != 0 {
        Err("error with ioctl FBIOGET_FSCREENINFO".to_string())
    } else {
        Ok(finfo)
    }
}

// # disable screensaver
// ds.sh
// 
// # stop unnecessary services to save on battery
// initctl stop framework
// initctl stop lab126_gui
// initctl stop x
// 
// # run main meme program
// 
// # Check battery
// cat /sys/devices/system/yoshi_battery/yoshi_battery0/battery_capacity
// 
// # Set brightness to lowest setting
// echo -n 0 > /sys/devices/system/fl_tps6116x/fl_tps6116x0/fl_intensity

// upload new meme:
// * webpage for meme_board is loaded and GET request is made for battery status
// * from webpage, XMLHttpRequest is made to upload image
// * on server, image data is written to file and meme_id is incremented

// while true; do
//         echo "doing meme work"
//         ./kindle_rust
//         echo +60 > /sys/devices/platform/pmic_rtc.1/rtc/rtc1/wakealarm
//         echo "doing process sleep to allow system work"
//         sleep 30
//         echo "sleeping"
//         echo mem > /sys/power/state
// done

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    loop {
        match maybe_update_meme() {
            Ok(updated) => {
                if updated {
                    log("updated meme");
                } else {
                    log("no update necessary");
                }
            },
            Err(e) => {
                log(&format!("Error: {}", e));
            }
        }

        kindle_sleep(Duration::from_secs(30), Duration::from_secs(5 * 60))
            .unwrap_or_else(|e| log(&format!("Error putting kindle to sleep: {}", e)));
    }
}

fn log(msg: &str) {
    println!("[{}] {}", Local::now(), msg);
}

fn kindle_sleep(process_sleep: Duration, deep_sleep: Duration) -> Result<(), String> {
    log(&format!("process sleep for {} seconds to allow system work", process_sleep.as_secs()));
    sleep(process_sleep);

    log(&format!("setting alarm for {} seconds in the future", deep_sleep.as_secs()));
    let wakealarm_path = "/sys/devices/platform/pmic_rtc.1/rtc/rtc1/wakealarm";
    fs::write(wakealarm_path, format!("+{}", deep_sleep.as_secs()))
        .map_err(|e| format!("Error writing to {}: {}", wakealarm_path, e))?;

    log("deep sleeping");
    let power_state_path = "/sys/power/state";
    fs::write(power_state_path, "mem")
        .map_err(|e| format!("Error writing to {}: {}", "/sys/power/state", e))?;

    log("waking up");
    Ok(())
}

fn maybe_update_meme() -> Result<bool, String> {
    let (server_meme_id, meme_bytes) = update_battery_status_and_get_meme()
        .map_err(|e| format!("Error updating status and getting meme id and bytes: {}", e))?;
    log(&format!("server_meme_id is {} and meme_bytes.len() is {}", server_meme_id, meme_bytes.len()));

    if server_meme_id == -1 {
        log("server meme id is -1, exiting");
        exit(0);
    }
    LOCAL_MEME_ID.with(|local_meme_id_cell| {
        let mut local_meme_id = local_meme_id_cell.borrow_mut();
        log(&format!("server_meme_id is {}, local_meme_id is {:?}", server_meme_id, local_meme_id));
        if local_meme_id.is_none() || local_meme_id.expect("id") != server_meme_id {
            let img = load_from_memory(&meme_bytes)
                .map_err(|e| format!("Error loading PNG from buffer with length {}: {}", meme_bytes.len(), e))?;
            update_meme(img, server_meme_id, &mut local_meme_id)
                .map_err(|e| format!("Error updating meme: {}", e))?;
            Ok(true)
        } else {
            Ok(false)
        }
    })
}

fn update_battery_status_and_get_meme() -> Result<(i32, Vec<u8>)/*i32*/, String> {
    let battery_percent_path = "/sys/devices/system/yoshi_battery/yoshi_battery0/battery_capacity";
    let battery_percent_raw = fs::read_to_string(battery_percent_path)
        .map_err(|e| format!("Error reading battery percentage from {}: {}", battery_percent_path, e))?;
    let battery_percent = battery_percent_raw.trim().trim_matches('%');

    LOCAL_MEME_ID.with(|local_meme_id_cell| {
        let local_meme_id = match *local_meme_id_cell.borrow_mut() {
            Some(id) => id.to_string(),
            None => String::new(),
        };

        let response_bytes: Vec<u8> = Command::new("curl")
            .arg("--data")
            .arg(format!("{} {}", battery_percent, local_meme_id))
            .arg("http://garspace.com/metrics/api/meme_status")
            .output()
            .map_err(|e| format!("Error to executing curl to get meme id: {}", e))?
            .stdout;

        let mut parts = response_bytes.splitn(2, |x| *x == '\n' as u8);
        let server_meme_id_bytes = parts.next()
            .ok_or("Expected split chunk with meme id but got nothing")?;
        let foo = str::from_utf8(server_meme_id_bytes)
            .map_err(|e| format!("Error converting output to utf8: {}", e))?;
        let server_meme_id = foo
            .parse::<i32>()
            .map_err(|e| format!("Error parseing meme id of '{}' to i32: {}", foo, e))?;
        let meme_bytes = parts.next()
            .ok_or("Expected split chunk with meme id but got nothing")?
            .to_vec();
        Ok((server_meme_id, meme_bytes))
    })
}

fn clear_screen() -> Result<(), String> {
    Command::new("eips")
        .arg("-l")
        .output()
        .map_err(|e| format!("failed to execute eips: {}", e))?;

    sleep(Duration::from_millis(1000));

    Command::new("eips")
        .arg("-c")
        .output()
        .map_err(|e| format!("failed to execute eips: {}", e))?;

    sleep(Duration::from_millis(1000));

    Ok(())
}

fn update_meme(img: DynamicImage, server_meme_id: i32, local_meme_id: &mut Option<i32>) -> Result<(), String> {
    clear_screen()
        .map_err(|e| format!("Error clearing screen: {}", e))?;

    let fb_path = "/dev/fb0";
    let fb = OpenOptions::new()
        .read(true)
        .write(true)
        // .create(true)
        .open(fb_path)
        .map_err(|e| format!("Error in opening framebuffer at {}: {}", fb_path, e))?;

    // println!("getting info...");
    let vinfo = get_vscreen_info(fb.as_raw_fd())
        .map_err(|e| format!("Error in get_vscreen_info: {}", e))?;
    let finfo = get_fscreen_info(fb.as_raw_fd())
        .map_err(|e| format!("Error in get_fscreen_info: {}", e))?;
    // println!("done");

    // println!("{:#?}", vinfo);
    // println!("{:#?}", finfo);

    let screensize = (vinfo.yres * finfo.line_length) as usize;
    // println!("screensize is {}", screensize);
    let mut fb_mmap: MmapMut = unsafe {
        MmapOptions::new()
            .len(screensize)
            .map_mut(&fb)
            .map_err(|e| format!("error mmap-ing fb: {}", e))?
    };
    let fb_ptr: *mut u8 = fb_mmap.as_mut_ptr();

    let mut update_info: MxcfbUpdateData51 = unsafe { mem::zeroed() };
    update_info.waveform_mode = 2;
    update_info.update_marker = 1;
    update_info.temp = 0x1001;
    update_info.update_region.width = vinfo.xres;
    update_info.update_region.height = vinfo.yres;

    let bw_img = img.to_luma();

    draw_img(
        fb_ptr,
        fb.as_raw_fd(),
        &mut update_info,
        &bw_img,
        finfo.line_length,
        vinfo.yres)
    .map_err(|e| format!("Error drawing image: {}", e))?;

    sleep(Duration::from_millis(2000));

    *local_meme_id = Some(server_meme_id);
    Ok(())
}

fn draw_img(
    fb_ptr: *mut u8,
    fb_desc: i32,
    update_info: &mut MxcfbUpdateData51,
    img_buf: &ImageBuffer<Luma<u8>, Vec<u8>>,
    width: u32,
    height: u32) -> Result<(), String>
{
    for y in 0..min(img_buf.height(), height) {
        for x in 0..min(img_buf.width(), width) {
            let px = img_buf.get_pixel(x, y);
            let idx = (x + y * width) as isize;
            // println!("setting x={} and y={} with idx {}", x, y, idx);
            unsafe {
                *fb_ptr.offset(idx) = px.channels()[0];
            }
        }
    }

    refresh(fb_desc, update_info)
        .map_err(|e| format!("Error refreshing screen: {}", e))
}

fn refresh(fb_desc: i32, update_info: &mut MxcfbUpdateData51) -> Result<(), String> {
    let result = unsafe {
        ioctl(fb_desc, 0x4048462e, update_info)
    };
    if result != 0 {
        Err("error refreshing".to_string())
    } else {
        Ok(())
    }
}
