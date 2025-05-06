use aw_inbox_rust::{mount_rocket, db};
use std::sync::{Arc, Mutex};

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let config = rocket::Config {
        port: 5600,
        address: "0.0.0.0".parse().unwrap(),
        ..Default::default()
    };
    println!("[DEBUG] Rocket config: address={:?}, port={:?}", config.address, config.port);

    // 使用固定的数据库路径
    let db_path = "inbox.db";
    
    // 先迁移数据库
    aw_inbox_rust::migrate_db(db_path).await.expect("数据库迁移失败");
    
    // 初始化数据库连接池
    let pool = db::init_pool().await.expect("数据库连接失败");
    let db = Arc::new(Mutex::new(pool));

    let _ = mount_rocket(rocket::custom(config), db)
        .launch()
        .await?;
    Ok(())
}
