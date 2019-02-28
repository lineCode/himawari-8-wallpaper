#[macro_use]
extern crate lazy_static;
#[macro_use] extern crate log;
extern crate android_logger;
use log::Level;
const LEVEL:Level = Level::Debug;

use jni::{JNIEnv};
use jni::objects::{JObject, JString, JClass, JValue};
use jni::sys::{jint, jbyteArray};

include!("../../../desktop/src/himawari8.rs");
include!("../../../desktop/src/wallpaper.rs");

//JNI加载完成
#[no_mangle]
pub extern fn JNI_OnLoad(_vm: jni::JavaVM, _reserved: *mut std::ffi::c_void) -> jint{
	android_logger::init_once(android_logger::Filter::default().with_min_level(LEVEL), Some("lib_wallpaper"));
	info!("JNI_OnLoad.");

	jni::sys::JNI_VERSION_1_6
}

#[no_mangle]
pub extern fn Java_io_github_planet0104_h8w_MainActivity_init<'a>(env: JNIEnv, _activity: JClass, activity:JObject){
	info!("init..");
	// if result.is_err(){
	// 	let err = result.err();
	// 	error!("{:?}", &err);
	// 	let _ = env.throw_new("java/lang/Exception", format!("{:?}", err));
	// }
}