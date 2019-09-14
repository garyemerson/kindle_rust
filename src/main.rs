use std::cell::RefCell;
use std::cmp::min;
use std::env::args;
use std::fs::{self, OpenOptions};
use std::io::{Read, stdin, stdout, Write};
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
use rand::{thread_rng, Rng};

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

fn foobar(waveform_mode: u32, update_mode: u32, update_marker: u32, ) -> Result<(), String> {
    clear_screen()
        .map_err(|e| format!("Error clearing screen: {}", e))?;

    let fb_path = "/dev/fb0";
    let fb = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fb_path)
        .map_err(|e| format!("Error in opening framebuffer at {}: {}", fb_path, e))?;
    let vinfo = get_vscreen_info(fb.as_raw_fd())
        .map_err(|e| format!("Error in get_vscreen_info: {}", e))?;
    let finfo = get_fscreen_info(fb.as_raw_fd())
        .map_err(|e| format!("Error in get_fscreen_info: {}", e))?;
    let screensize = (vinfo.yres * finfo.line_length) as usize;
    let mut fb_mmap: MmapMut = unsafe {
        MmapOptions::new()
            .len(screensize)
            .map_mut(&fb)
            .map_err(|e| format!("error mmap-ing fb: {}", e))?
    };
    let fb_ptr: *mut u8 = fb_mmap.as_mut_ptr();

    let width = finfo.line_length;
    let height = vinfo.yres;
    println!("width: {}, height: {}", width, height);

    gmplay8(fb_ptr, fb.as_raw_fd(), vinfo).expect("gmplay8");

    // let frames: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = (0..60)
    //     .map(|n| {
    //         let path = format!("frames/frame{}.png", n);
    //         println!("loading frame {}", n);
    //         image::open(&path)
    //             .expect(&format!("open image {}", path))
    //             .to_luma()
    //     })
    //     .collect();
    // println!("{} frames", frames.len());

    // loop {
    //     print!("waveform_mode: ");
    //     stdout().flush().expect("flush");
    //     let mut s = String::new();
    //     stdin().read_line(&mut s).expect("Did not enter a correct string");
    //     let waveform_mode = s.trim().parse::<u32>().expect("parse u32 from input");
    //     for f in frames.clone() {
    //         let frame_timer = std::time::Instant::now();
    //         let mut update_info: MxcfbUpdateData51 = unsafe { mem::zeroed() };
    //         update_info.waveform_mode = waveform_mode /*1*/ /*257*/ /*4*/ /*2*/ /*3*/ /*6*/;
    //         update_info.update_mode = update_mode;
    //         update_info.update_marker = update_marker /*1*/;
    //         update_info.temp = 0x1001;
    //         update_info.update_region.width = vinfo.xres;
    //         update_info.update_region.height = vinfo.yres;

    //         let timer = std::time::Instant::now();
    //         draw_img(fb_ptr, fb.as_raw_fd(), &mut update_info, &f, width, height)
    //             .map_err(|e| format!("Error drawing image: {}", e))?;
    //         println!("{}ms", timer.elapsed().as_millis());
    //         let draw_time = frame_timer.elapsed().as_millis();
    //         if draw_time > 50 {
    //             println!("{}ms, blew frame budget :(", draw_time);
    //         } else {
    //             sleep(Duration::from_millis(50 as u64 - draw_time as u64));
    //         }
    //     }
    // }

    // for _ in 0..120 {
    //     let x_offset = thread_rng().gen_range(0, width - 100);
    //     let y_offset = thread_rng().gen_range(0, height - 100);
    //     foo_draw(fb_ptr, fb.as_raw_fd(), vinfo, width, height, x_offset, y_offset)
    //         .map_err(|e| format!("error foo_draw: {}", e))?;
    // }

    Ok(())
}

struct IterChunks<T> {
    inner: Box<Iterator<Item=T>>,
    chunk_size: usize,
}
impl<T> Iterator for IterChunks<T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Vec<T>> {
        let mut v = Vec::new();
        while v.len() != self.chunk_size {
            match self.inner.next() {
                Some(x) => v.push(x),
                None => break,
            }
        }
        if v.len() == 0 {
            None
        } else {
            Some(v)
        }
    }
}

fn gmplay8(fb_ptr: *mut u8, fb_desc: i32, vinfo: FbVarScreenInfo) -> Result<(), String> {
    let mut byte_stream = stdin();
        // .bytes();
        // .map(|mabye_byte| mabye_byte.expect("byte"));
    // let frames = IterChunks { inner: Box::new(byte_stream), chunk_size: (600 * 800) / 8};
    let timer = std::time::Instant::now();
    let mut frame_count = 0;
    let mut frame_load_time = std::time::Instant::now();
    // for (f_num, frame) in frames.enumerate() {
    let mut frame = vec![0; (600 * 800) / 8];
    while let Ok(_) = byte_stream.read_exact(&mut frame) {
        println!("{}ms to load frame", frame_load_time.elapsed().as_millis());
        let frame_timer = std::time::Instant::now();
        println!("drawing frame {} with {} bytes", frame_count, frame.len());
        let chunk_size: usize = /*vinfo.xres_virtual*/ 600 / 8;
        // let rows = IterChunks { inner: Box::new(frame.into_iter()), chunk_size: chunk_size };
        for (row_num, row) in frame.chunks(chunk_size).enumerate() { //rows.enumerate() {
            // println!("chunk_size is {}", chunk_size);
            // println!("drawing row {}", row_num);
            for (i, byte) in row.iter().enumerate() {
                let fb_idx = (row_num * vinfo.xres_virtual as usize) as isize + i as isize * 8;
                // println!("writing row chunk {} with fb_idx {}", i, fb_idx);
                let mut byte_copy = *byte;

                unsafe {
                    *fb_ptr.offset(fb_idx) = (byte_copy & 1) * 255;
                    byte_copy >>= 1;
                    *fb_ptr.offset(fb_idx + 1) = (byte_copy & 1) * 255;
                    byte_copy >>= 1;
                    *fb_ptr.offset(fb_idx + 2) = (byte_copy & 1) * 255;
                    byte_copy >>= 1;
                    *fb_ptr.offset(fb_idx + 3) = (byte_copy & 1) * 255;
                    byte_copy >>= 1;
                    *fb_ptr.offset(fb_idx + 4) = (byte_copy & 1) * 255;
                    byte_copy >>= 1;
                    *fb_ptr.offset(fb_idx + 5) = (byte_copy & 1) * 255;
                    byte_copy >>= 1;
                    *fb_ptr.offset(fb_idx + 6) = (byte_copy & 1) * 255;
                    byte_copy >>= 1;
                    *fb_ptr.offset(fb_idx + 7) = (byte_copy & 1) * 255;
                }
            }
        }

        let mut update_info: MxcfbUpdateData51 = unsafe { mem::zeroed() };
        update_info.waveform_mode = 1 /*4*/ /*2*/;
        update_info.update_marker = 1;
        update_info.temp = 0x1001;
        update_info.update_region.width = vinfo.xres;
        update_info.update_region.height = vinfo.yres;

        refresh(fb_desc, &mut update_info)
            .map_err(|e| format!("Error refreshing screen: {}", e))?;
        frame_count += 1;

        let draw_time = frame_timer.elapsed().as_millis();
        if draw_time > 50 {
            println!("{}ms, blew frame budget :(", draw_time);
        } else {
            sleep(Duration::from_millis(50 as u64 - draw_time as u64));
        }
        // let draw_time = frame_timer.elapsed().as_millis();
        // if draw_time > 50 {
        //     println!("{}ms, blew frame budget :(", draw_time);
        // } else {
        //     while frame_timer.elapsed().as_millis() < 50 {
        //         sleep(Duration::from_millis(1));
        //     }
        // }
        frame_load_time = std::time::Instant::now();
    }
    println!("{} frames in {}s", frame_count, timer.elapsed().as_millis() as f64 / 1000.0);

    Ok(())

    // u32 i, x, y, b, fbsize = FBSIZE;
    // u8 fbt[FBSIZE];
    // while (fread(fbt, fbsize, 1, stdin)) {
    //     teu += 50; // teu: next update time
    //     // if (getmsec() > teu + 1000) continue; // drop frame if > 1 sec behind
    //     gmlib(GMLIB_VSYNC); // wait for fb0 ready
    //     for (y = 0; y < 800; y++)
    //         for (x = 0; x < 600; x += 8) {
    //             b = fbt[600 / 8 * y + x / 8];
    //             i = y * fs + x;
    //             fb0[i] = (b & 1) * 255;
    //             b >>= 1;
    //             fb0[i + 1] = (b & 1) * 255;
    //             b >>= 1;
    //             fb0[i + 2] = (b & 1) * 255;
    //             b >>= 1;
    //             fb0[i + 3] = (b & 1) * 255;
    //             b >>= 1;
    //             fb0[i + 4] = (b & 1) * 255;
    //             b >>= 1;
    //             fb0[i + 5] = (b & 1) * 255;
    //             b >>= 1;
    //             fb0[i + 6] = (b & 1) * 255;
    //             b >>= 1;
    //             fb0[i + 7] = (b & 1) * 255;
    //         }
    //     fc++;
    //     gmlib(GMLIB_UPDATE);
    // }
}

fn foo_draw(
    fb_ptr: *mut u8,
    fb_desc: i32,
    vinfo: FbVarScreenInfo,
    width: u32,
    height: u32,
    x_offset: u32,
    y_offset: u32) -> Result<(), String>
{
    let mut update_info: MxcfbUpdateData51 = unsafe { mem::zeroed() };
    update_info.waveform_mode = 4 /*2*/;
    update_info.update_marker = 1;
    update_info.temp = 0x1001;
    update_info.update_region.width = vinfo.xres;
    update_info.update_region.height = vinfo.yres;
    for y in 0..100 {
        for x in 0..100 {
            let idx = ((x + x_offset) + (y + y_offset) * width) as isize;
            unsafe {
                *fb_ptr.offset(idx) = 0;
            }
        }
    }

    // println!("refreshing");
    let timer = std::time::Instant::now();
    refresh(fb_desc, &mut update_info)
        .map_err(|e| format!("Error refreshing screen: {}", e))?;
    println!("{}ms", timer.elapsed().as_millis());

    Ok(())
}

fn main() {
    println!("foo");
    let waveform_mode = args().nth(1).expect("waveform_mode arg").parse::<u32>().expect("parse u32");
    let update_mode = args().nth(2).expect("update_mode arg").parse::<u32>().expect("parse u32");
    let update_marker = args().nth(3).expect("update_marker arg").parse::<u32>().expect("parse u32");
    foobar(waveform_mode, update_mode, update_marker).expect("foobar");
    return;

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
