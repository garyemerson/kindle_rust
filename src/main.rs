extern crate memmap;
extern crate libc;
extern crate image;
extern crate chrono;

// use std::cmp::min;
// use std::env;
// use std::io::prelude::*;
// use std::net::TcpStream;
use std::time::{/*Instant, SystemTime,*/ Duration};
use std::fs::{self, OpenOptions};
use std::mem;
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::str;
use std::thread::sleep;
use image::imageops::{resize /*, brighten*/};
use image::{ImageBuffer, Luma, DynamicImage, Pixel, FilterType, load_from_memory};
use libc::ioctl;
use memmap::{MmapOptions, MmapMut};
use chrono::Local;


#[repr(C)]
#[derive(Default, Debug)]
struct FbBitField {
    offset: u32,      /* beginning of bitfield */
    length: u32,      /* length of bitfield */
    msb_right: u32,   /* !=0: Most significant bit is right */
}

#[repr(C)]
#[derive(Default, Debug)]
struct FbVarScreenInfo {
    xres: u32,           /* visible resolution       */
    yres: u32,
    xres_virtual: u32,   /* virtual resolution       */
    yres_virtual: u32,
    xoffset: u32,        /* offset from virtual to visible */
    yoffset: u32,        /* resolution           */

    bits_per_pixel: u32, /* guess what           */
    grayscale: u32,      /* != 0 Graylevels instead of colors */

    red: FbBitField,     /* bitfield in fb mem if true color, */
    green: FbBitField,   /* else only length is significant */
    blue: FbBitField,
    transp: FbBitField,  /* transparency         */  

    nonstd: u32,         /* != 0 Non standard pixel format */

    activate: u32,       /* see FB_ACTIVATE_*        */

    height: u32,         /* height of picture in mm    */
    width: u32,          /* width of picture in mm     */

    accel_flags: u32,    /* (OBSOLETE) see fb_info.flags */

    /* Timing: All values in pixclocks, except pixclock (of course) */
    pixclock: u32,       /* pixel clock in ps (pico seconds) */
    left_margin: u32,    /* time from sync to picture    */
    right_margin: u32,   /* time from picture to sync    */
    upper_margin: u32,   /* time from sync to picture    */
    lower_margin: u32,
    hsync_len: u32,      /* length of horizontal sync    */
    vsync_len: u32,      /* length of vertical sync  */
    sync: u32,           /* see FB_SYNC_*        */
    vmode: u32,          /* see FB_VMODE_*       */
    rotate: u32,         /* angle we rotate counter clockwise */
    reserved: [u32; 5]   /* Reserved for future compatibility */
}

#[repr(C)]
#[derive(Default, Debug)]
struct FbFixScreenInfo {
    id: [u8; 16],       /* identification string eg "TT Builtin" */
    smem_start: usize,    /* Start of frame buffer mem */
                        /* (physical address) */
    smem_len: u32,      /* Length of frame buffer mem */
    type_: u32,         /* see FB_TYPE_*        */
    type_aux: u32,      /* Interleave for interleaved Planes */
    visual: u32,        /* see FB_VISUAL_*      */ 
    xpanstep: u16,      /* zero if no hardware panning  */
    ypanstep: u16,      /* zero if no hardware panning  */
    ywrapstep: u16,     /* zero if no hardware ywrap    */
    line_length: u32,   /* length of a line in bytes    */
    mmio_start: usize,    /* Start of Memory Mapped I/O   */
                        /* (physical address) */
    mmio_len: u32,      /* Length of Memory Mapped I/O  */
    accel: u32,         /* Indicate to driver which */
                        /*  specific chip/card we have  */
    reserved: [u16; 3]  /* Reserved for future compatibility */
}

#[repr(C)]
#[derive(Default, Debug)]
struct MxcfbRect {
    top: u32,
    left: u32,
    width: u32,
    height: u32
}

#[repr(C)]
#[derive(Default, Debug)]
struct MxcfbAltBufferData {
    phys_addr: u32,
    width: u32,
    height: u32,
    alt_update_region: MxcfbRect
}

#[repr(C)]
#[derive(Default, Debug)]
struct MxcfbUpdateData51 {
    update_region: MxcfbRect,
    waveform_mode: u32,
    update_mode: u32,
    update_marker: u32,
    hist_bw_waveform_mode: u32,
    hist_gray_waveform_mode: u32,
    temp: i32,
    flags: u32,
    alt_buffer_data: MxcfbAltBufferData
}

const MEME_ID_PATH: &'static str = "/mnt/us/garspace/workspaces/meme_board/meme_id";


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

// kindle update:
// * POST to send battery status and get meme_id
// * from meme_id determine if udpate needed
// * if yes then
// *    GET request for meme image
// *    draw meme
// * go to sleep for 5 min

fn main() {
    // let url = env::args().nth(1).expect("Must supply arg for url");
    // let output_raw = Command::new("curl")
    //     .arg(url)
    //     .output()
    //     .expect("failed to execute curl")
    //     .stdout;
    // let output = str::from_utf8(&output_raw)
    //     .expect("output to utf8");
    // println!("output is:\n'{}'", output);

    loop {
        match maybe_update_meme() {
            Ok(updated) => {
                if updated {
                    log("updated meme");
                } else {
                    log("update not necessary");
                }
            },
            Err(e) => {
                log(&format!("Error: {}", e));
            }
        }

        kindle_sleep(5 * 60)
            .unwrap_or_else(|e| log(&format!("Error putting kindle to sleep: {}", e)));
    }
}

fn log(msg: &str) {
    println!("[{}] {}", Local::now(), msg);
}

fn kindle_sleep(seconds: i32) -> Result<(), String> {
    log(&format!("sleeping and setting alarm for {} seconds in the future", seconds));

    let wakealarm_path = "/sys/devices/platform/pmic_rtc.1/rtc/rtc1/wakealarm";
    fs::write(wakealarm_path, format!("+{}", seconds))
        .map_err(|e| format!("Error writing to {}: {}", wakealarm_path, e))?;

    let power_state_path = "/sys/power/state";
    fs::write(power_state_path, "mem")
        .map_err(|e| format!("Error writing to {}: {}", "/sys/power/state", e))?;

    log("waking up");
    Ok(())
}

fn maybe_update_meme() -> Result<bool, String> {
    let server_meme_id = update_battery_status_and_get_meme_id()
        .map_err(|e| format!("Error updating status and getting meme id: {}", e))?;

    let local_meme_id_raw = fs::read_to_string(MEME_ID_PATH)
        .map_err(|e| format!("Error reading local meme id from {}: {}", MEME_ID_PATH, e))?;
    let local_meme_id = local_meme_id_raw.trim().parse::<i32>()
        .map_err(|e| format!("Error parsing '{}' as i32: {}", local_meme_id_raw, e))?;

    log(&format!("server_meme_id is {}, local_meme_id is {}", server_meme_id, local_meme_id));
    if local_meme_id != server_meme_id {
        let output_raw: Vec<u8> = Command::new("curl")
            .arg("http://garspace.com/metrics/api/meme")
            .output()
            .map_err(|e| format!("Error to executing curl to get meme: {}", e))?
            .stdout;
        let img = load_from_memory(&output_raw)
            .map_err(|e| format!("Error loading PNG from buffer with length {}: {}", output_raw.len(), e))?;
        update_meme(img, server_meme_id)
            .map_err(|e| format!("Error updating meme: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn update_battery_status_and_get_meme_id() -> Result<i32, String> {
    let battery_percent_path = "/sys/devices/system/yoshi_battery/yoshi_battery0/battery_capacity";
    let battery_percent_raw = fs::read_to_string(battery_percent_path)
        .map_err(|e| format!("Error reading battery percentage from {}: {}", battery_percent_path, e))?;
    let battery_percent = battery_percent_raw.trim().trim_matches('%');

    // curl --data 97 http://garspace.com/metrics/api/meme_status
    let output_raw = Command::new("curl")
        .arg("--data")
        .arg(battery_percent)
        .arg("http://garspace.com/metrics/api/meme_status")
        .output()
        .map_err(|e| format!("Error to executing curl to get meme id: {}", e))?
        .stdout;
    let output = str::from_utf8(&output_raw)
        .map_err(|e| format!("Error converting output to utf8: {}", e))?
        .trim();
    output.parse::<i32>()
        .map_err(|e| format!("Error converting output '{}' to i32: {}", output, e))
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

    Ok(())
}

fn update_meme(img: DynamicImage, meme_id: i32) -> Result<(), String> {
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
    // println!("mmaping...");
    let mut fb_mmap: MmapMut = unsafe {
        MmapOptions::new()
            .len(screensize)
            .map_mut(&fb)
            .map_err(|e| format!("error mmap-ing fb: {}", e))?
    };
    // println!("done");
    let fb_ptr: *mut u8 = fb_mmap.as_mut_ptr();

    // println!("vinfo.xres {}", vinfo.xres);
    // for x in 100..200 {
    //     for y in 100..200 {
    //         let idx = (x + y * finfo.line_length) as isize;
    //         // println!("idx is {}", idx);
    //         unsafe {
    //             *fb_ptr.offset(idx) = 50u8;
    //         }
    //     }
    // }

    let mut update_info: MxcfbUpdateData51 = unsafe { mem::zeroed() };
    update_info.waveform_mode = 2;
    update_info.update_marker = 1;
    update_info.temp = 0x1001;
    update_info.update_region.width = vinfo.xres;
    update_info.update_region.height = vinfo.yres;
    // unsafe {
    //     refresh(fb.as_raw_fd(), &mut update_info);
    // }

    // println!("opening img...");
    // let path = env::args().nth(1)
    //     .ok_or("Must provide arg for image file")?;
    // let img: DynamicImage = image::open(&path)
    //     .map_err(|e| format!("Error opening image {}: {}", path, e))?;
    let bw_img = img.to_luma();
    // println!("done");

    draw_img(
        fb_ptr,
        fb.as_raw_fd(),
        &mut update_info,
        &bw_img,
        finfo.line_length,
        vinfo.yres)
    .map_err(|e| format!("Error drawing image: {}", e))?;

    sleep(Duration::from_millis(1000));

    fs::write(MEME_ID_PATH, meme_id.to_string().into_bytes())
        .map_err(|e| format!("Error updating local meme id to {}: {}", meme_id, e))
}

fn draw_img(
    fb_ptr: *mut u8,
    fb_desc: i32,
    update_info: &mut MxcfbUpdateData51,
    img_buf: &ImageBuffer<Luma<u8>, Vec<u8>>,
    width: u32,
    height: u32) -> Result<(), String>
{
    let mut new_width = width;
    let mut new_height = height;
    if img_buf.width() != width || img_buf.height() != height {
        let img_ratio = img_buf.width() as f32 / img_buf.height() as f32;
        let scr_ratio = width as f32 / height as f32;
        if img_ratio > scr_ratio { // img wider than screen
            let ratio: f32 = width as f32 / img_buf.width() as f32;
            new_width = width;
            new_height = (img_buf.height() as f32 * ratio) as u32;
        } else { // img taller than screen
            let ratio: f32 = height as f32 / img_buf.height() as f32;
            new_width = (img_buf.width() as f32 * ratio) as u32;
            new_height = height;
        };
    }

    log(&format!("resizing to {}w x {}h", new_width, new_height));
    let resized_img = resize(img_buf, new_width, new_height, FilterType::CatmullRom);
    // log("brightening image");
    // let bright_img = brighten(&resized_img, 25);

    for y in 0..new_height {
        for x in 0..new_width {
            let px = resized_img.get_pixel(x, y);
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

// fn make_http_request() {
//     let mut stream = TcpStream::connect("69.162.69.148:80")
//         .expect("Failed to open tcp socket");

//     let _ = stream.write(
//             "GET / HTTP/1.1\r\n\
//             Host: icanhazip.com\r\n\r\n".as_bytes())
//         .expect("write failed");
//     let mut response = String::new();
//     let _ = stream.read_to_string(&mut response)
//         .expect("read failed");
//     println!("response:\n{}", response);
// }
