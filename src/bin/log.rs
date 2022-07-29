use chrono::Local;

pub fn log(logtxt: &str) {
    let now = format!("{:?}", Local::now());
    println!("{}  {}", now, logtxt);
}
