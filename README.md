# aw-inbox

多语言笔记收集与管理工具，支持 Rust 与 Python 实现，便于 ActivityWatch 等系统集成。

## 功能简介
- **RESTful API**：基于 Rust（axum + sqlx）实现，支持笔记的增删改查（CRUD）、批量获取、标签管理等。
- **Python 脚本**：用于数据处理、辅助自动化。
- **数据库迁移脚本**：支持结构初始化与升级。

## 技术栈
- Rust (axum, sqlx)
- Python 3
- SQLite

## 快速开始
### 1. 克隆仓库
```bash
git clone git@github.com:PT123123/aw-inbox.git
cd aw-inbox
```

### 2. Rust 后端运行
确保已安装 Rust 和 SQLite3。
```bash
cargo run --release
```

### 3. Python 脚本
```bash
python3 aw_inbox/main.py
```

### 4. 数据库迁移
可用 migrations 目录下的 SQL 脚本初始化表结构。

### 5. 测试
- Rust 集成测试：
  ```bash
  cargo test
  ```
- Python 测试：
  ```bash
  pytest tests/
  ```

## 目录结构
```
aw-inbox/
├── src/             # Rust 源码
├── tests/           # Rust/Python 测试
├── aw_inbox/        # Python 脚本
├── migrations/      # 数据库迁移脚本
├── dist/            # Python 打包产物（已忽略）
├── ...
```

## 贡献与开发
欢迎 issue 和 PR！建议先提 issue 讨论。

## License
MIT
