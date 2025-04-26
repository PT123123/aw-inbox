import sqlite3
import logging
import os
from pathlib import Path
from flask import Flask, request, jsonify
import requests
from flask_cors import CORS

logger = logging.getLogger(__name__)

class SqliteStorage:
"""简单的SQLite存储实现"""
def __init__(self, testing=False, filepath=None):
self.filepath = filepath or str(Path.home() / ".activitywatch" / "flomo.db")
self.conn = sqlite3.connect(self.filepath)

def create_table(self, table_name, schema):
cursor = self.conn.cursor()
cursor.execute(f"CREATE TABLE IF NOT EXISTS {table_name} ({schema})")
self.conn.commit()

class Flomo:
def __init__(self, storage_strategy=None, testing=False, **kwargs):
self.logger = logger.getChild("Flomo")
if storage_strategy is None:
data_dir = str(Path.home() / ".activitywatch")
os.makedirs(data_dir, exist_ok=True)

filepath = os.path.join(data_dir, "flomo.db")
self.storage_strategy = SqliteStorage(testing=testing, filepath=filepath)
self._create_tables()
else:
self.storage_strategy = storage_strategy(testing=testing, **kwargs)

def _create_tables(self):
# 创建表的逻辑
pass

def test_connection(self):
try:
conn = sqlite3.connect(self.storage_strategy.filepath)
conn.execute("SELECT 1")
conn.close()
print("数据库连接成功")
except Exception as e:
print("数据库连接失败:", e)

app = Flask(__name__)
CORS(app, origins=["http://localhost:5600"])  # 允许 http://localhost:5600 访问

# 配置日志
logging.basicConfig(level=logging.INFO)

# 示例路由
@app.route('/')
def index():
logger.info("Request received at /")
return "Hello, World!"

@app.route('/flomo/notes')
def get_notes():
logger.info("Request received at /flomo/notes")
return "Here are your notes."

# 打印所有路由
def print_routes():
logger.info("Printing all routes...")  # 调试信息
for rule in app.url_map.iter_rules():
logger.info(f"Route: {rule.rule} -> {rule.endpoint}")

if __name__ == '__main__':
print_routes()  # 确保在 app.run() 之前调用
app.run(host='0.0.0.0', port=5601, debug=False)  # 禁用调试模式