use crate::himawari8;
use chrono::{DateTime, Datelike, Local, NaiveDateTime, Timelike, Utc};
use image::GenericImage;
use image::{ImageBuffer, Rgb};
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use wallpaper;

const INFO_DOWNLOADING: &str = "正在下载中，请稍后";

lazy_static! {
    static ref DOWNLOADING: Mutex<bool> = Mutex::new(false);
}

//dim 2 => 2x2图
//dim 4 => 4x4图
fn download<C>(
    dim: i32,
    callback: C,
) -> Result<Option<ImageBuffer<Rgb<u8>, Vec<u8>>>, Box<std::error::Error>>
where
    C: Fn(i32, i32) + 'static,
{
    if *DOWNLOADING.lock().unwrap() {
        return Ok(None);
    }
    *DOWNLOADING.lock().unwrap() = true;
    let mut timestamp = Utc::now().timestamp_millis();
    //减去20分钟
    timestamp -= 20 * 60 * 1000;
    let utc = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp / 1000, 0), Utc);

    //20分钟之前的
    let file_name = format!(
        "{}d_{}_{}_{}.png",
        dim,
        // utc.year(),
        // utc.month(),
        utc.day(),
        utc.hour(),
        utc.minute() / 10
    );

    //删除旧的文件
    let paths = std::fs::read_dir("").unwrap();
    let cur_2d = format!("2d_{}_{}_{}.png", utc.day(), utc.hour(), utc.minute() / 10);
    let cur_4d = format!("4d_{}_{}_{}.png", utc.day(), utc.hour(), utc.minute() / 10);
    for path in paths {
        let p = path.unwrap().path().display().to_string();
        if p != "icon.ico" && p != cur_2d && p != cur_4d && p != "wallpaper.png" && p != "conf.ini"
        {
            println!("删除文件:{}", p);
            let _ = std::fs::remove_file(p);
        }
    }

    if Path::new(&file_name).exists() {
        println!(
            "{:?} 文件已存在 直接返回{}",
            Local::now(),
            file_name
        );
        *DOWNLOADING.lock().unwrap() = false;
        Ok(Some(image::open(file_name)?.to_rgb()))
    } else {
        let image = if dim == 4 {
            himawari8::combine_4x4(
                utc.year(),
                utc.month(),
                utc.day(),
                utc.hour(),
                utc.minute(),
                callback,
            )?
        } else {
            himawari8::combine_2x2(
                utc.year(),
                utc.month(),
                utc.day(),
                utc.hour(),
                utc.minute(),
                callback,
            )?
        };
        image.save(file_name)?;
        *DOWNLOADING.lock().unwrap() = false;
        Ok(Some(image))
    }
}

//整幅画
pub fn set_full<C>(
    screen_width: i32,
    screen_height: i32,
    callback: C,
) -> Result<(), Box<std::error::Error>>
where
    C: Fn(i32, i32) + 'static,
{
    //判断屏幕 横屏(桌面), 竖屏(手机)

    /*
       1920x1200、1600x900显示器都下载1100x1100的图片
       横屏:
       如果屏幕高度小于1200,下载2x2图
       如果屏幕高度大于1200,下载4x4图

       竖屏:
       如果屏幕宽度小于1200，下载2x2图
       如果屏幕宽度大于1200，下载4x4图
    */
    let image = if std::cmp::min(screen_width, screen_height) < 1200 {
        download(2, callback)?
    } else {
        download(4, callback)?
    };
    if image.is_none() {
        println!("{}", INFO_DOWNLOADING);
        return Ok(());
    }
    let image = image.unwrap();

    //缩放
    let size = if screen_height < screen_width {
        //横屏取屏幕高度的87%作为地球边长, 留出任务栏，将图片缩放到指定高度，然后拼接到黑色背景。
        //桌面高度大于等于1200 高度87%， 小于1200高度95%
        let scale = if screen_height > 1200 { 0.87 } else { 0.95 };
        (screen_height as f64 * scale) as u32
    } else {
        //竖屏取屏幕宽度的100%作为地球边长
        screen_width as u32
    };
    let image = image::imageops::resize(&image, size, size, image::FilterType::Gaussian);
    // image.save("image.png").unwrap();

    let mut wallpaper: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::new(screen_width as u32, screen_height as u32);

    //拼接
    let offset_x = ((wallpaper.width() - image.width()) / 2) as usize;
    let top_border_scale = if screen_height > 1200 { 0.25 } else { 0.0 };
    let offset_y = ((wallpaper.height() - image.height()) as f64 * top_border_scale) as usize;
    let ew = image.width() as usize;
    let image = image.into_raw();
    for (y, buf) in image.chunks(ew * 3).enumerate() {
        let offset = screen_width as usize * 3 * (y + offset_y) + offset_x * 3;
        wallpaper
            .get_mut(offset..offset + buf.len())
            .unwrap()
            .copy_from_slice(buf);
    }

    wallpaper.save("wallpaper.png")?;
    wallpaper::set_from_path(absolute_path("wallpaper.png")?.to_str().unwrap())?;

    Ok(())
}

pub fn absolute_path<P>(path: P) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

//取半边, 由于半边要求地球图片不管是720p还是1080p，直径都大于1100，所以都取4x4图
pub fn set_half<C>(
    screen_width: i32,
    screen_height: i32,
    callback: C,
) -> Result<(), Box<std::error::Error>>
where
    C: Fn(i32, i32) + 'static,
{
    let image = download(4, callback)?;
    if image.is_none() {
        println!("{}", INFO_DOWNLOADING);
        return Ok(());
    }
    let image = image.unwrap();

    let mut wallpaper: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::new(screen_width as u32, screen_height as u32);

    // 缩放
    if screen_height < screen_width {
        //横屏: 地球直径取屏幕宽度，取地球上半部分
        let image = image::imageops::resize(
            &image,
            screen_width as u32,
            screen_width as u32,
            image::FilterType::Gaussian,
        );

        //拼接
        let offset_x = ((wallpaper.width() - image.width()) / 2) as usize;
        let offset_y = (screen_height as f64 * 0.03) as usize; //顶部添加一些边距
        let ew = image.width() as usize;
        let image = image.into_raw();
        for (y, buf) in image.chunks(ew * 3).enumerate() {
            if (y + offset_y) < wallpaper.height() as usize {
                let offset = screen_width as usize * 3 * (y + offset_y) + offset_x * 3;
                wallpaper
                    .get_mut(offset..offset + buf.len())
                    .unwrap()
                    .copy_from_slice(buf);
            } else {
                break;
            }
        }
    } else {
        //竖屏: 地球直径取屏幕高度，上午取地球右半部分，下午取地球左半部分
        let mut image = image::imageops::resize(
            &image,
            screen_height as u32,
            screen_height as u32,
            image::FilterType::Nearest,
        );

        use chrono::Local;
        let time = Local::now();
        if time.hour() <= 12 {
            //取地球右半部分
            let w = image.width() / 2;
            let x = image.width() - w;
            image = image.sub_image(x, 0, w, image.height()).to_image();
        } else {
            //取地球左半部分
            let w = image.width() / 2;
            image = image.sub_image(0, 0, w, image.height()).to_image();
        }

        //拼接
        let offset_x = ((wallpaper.width() - image.width()) / 2) as usize;
        let offset_y = ((wallpaper.height() - image.height()) / 2) as usize;
        let ew = image.width() as usize;
        let image = image.into_raw();
        for (y, buf) in image.chunks(ew * 3).enumerate() {
            if (y + offset_y) < wallpaper.height() as usize {
                let offset = screen_width as usize * 3 * (y + offset_y) + offset_x * 3;
                wallpaper
                    .get_mut(offset..offset + buf.len())
                    .unwrap()
                    .copy_from_slice(buf);
            } else {
                break;
            }
        }
    };

    wallpaper.save("wallpaper.png")?;
    wallpaper::set_from_path(absolute_path("wallpaper.png")?.to_str().unwrap())?;

    Ok(())
}