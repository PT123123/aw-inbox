import sqlite3
import logging
import os
from pathlib import Path
from flask import Flask, request, jsonify
import requests
from flask_cors import CORS
from datetime import datetime, timezone, timedelta
import json
from aw_core.models import Event

logger = logging.getLogger(__name__)

class SqliteStorage:
    """简单的SQLite存储实现"""
    def __init__(self, testing=False, filepath=None):
        self.filepath = filepath or str(Path.home() / ".activitywatch" / "inbox.db")
        self.conn = sqlite3.connect(self.filepath)

    def create_table(self, table_name, schema):
        cursor = self.conn.cursor()
        cursor.execute(f"CREATE TABLE IF NOT EXISTS {table_name} ({schema})")
        self.conn.commit()

class Inbox:
    def __init__(self, storage_strategy=None, testing=False, **kwargs):
        self.logger = logger.getChild("Inbox")
        if storage_strategy is None:
            data_dir = str(Path.home() / ".activitywatch")
            os.makedirs(data_dir, exist_ok=True)

            filepath = os.path.join(data_dir, "inbox.db")
            self.storage_strategy = SqliteStorage(testing=testing, filepath=filepath)
            self._create_tables()
        else:
            self.storage_strategy = storage_strategy(testing=testing, **kwargs)

    def _create_tables(self):
        conn = sqlite3.connect(self.storage_strategy.filepath)
        cursor = conn.cursor()

        # 新增notes表（关键修复）
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                tags TEXT DEFAULT '[]',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # 创建其他表（原有逻辑）
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                priority INTEGER DEFAULT 0
            )
        """)

        # 创建 comments 表
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                note_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            )
        """)

        conn.commit()
        conn.close()

    def test_connection(self):
        try:
            conn = sqlite3.connect(self.storage_strategy.filepath)
            conn.execute("SELECT 1")
            conn.close()
            print("数据库连接成功")
        except Exception as e:
            print("数据库连接失败:", e)

    def get_notes(self, limit=50, tag=None, created_after=None, created_before=None):
        """从数据库查询笔记（新增标签过滤）"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        # 构建基础查询
        query = """
            SELECT id, content, tags, created_at, updated_at
            FROM notes
            WHERE 1=1
        """
        params = []

        # 标签过滤（精确匹配）
        if tag:
            query += " AND EXISTS (SELECT 1 FROM json_each(tags) WHERE value = ?)"
            params.append(tag)

        # 时间范围过滤（保持不变）
        if created_after:
            query += " AND created_at >= ?"
            params.append(created_after.astimezone(timezone.utc).isoformat())
        if created_before:
            query += " AND created_at < ?"
            params.append(created_before.astimezone(timezone.utc).isoformat())

        # 排序和分页（保持不变）
        query += " ORDER BY created_at DESC LIMIT ?"
        params.append(limit)

        # 执行查询
        cursor.execute(query, params)
        rows = cursor.fetchall()

        # 转换为Event对象（保持不变）
        return [self._row_to_event(row) for row in rows]

    def _row_to_event(self, row):
        """将数据库行转换为Event对象"""
        return Event(
            timestamp=datetime.fromisoformat(row['created_at']).astimezone(timezone.utc),
            duration=timedelta(0),
            data={
                "id": row['id'],
                "content": row['content'],
                "tags": json.loads(row['tags']) if row['tags'] else [],
                "created_at": row['created_at'],
                "updated_at": row['updated_at']
            }
        )

    def create_note(self, content: str, tags: list = None, created: datetime = None):
        """创建新笔记并存入数据库"""
        self.logger.info(f"Inbox.create_note called with content: '{content}', tags: {tags}, created: {created}") # 添加日志
        conn = sqlite3.connect(self.storage_strategy.filepath)
        cursor = conn.cursor()

        # 处理参数
        tags_json = json.dumps(tags or [])
        created = created or datetime.now(timezone.utc)

        # 执行插入
        cursor.execute("""
            INSERT INTO notes (content, tags, created_at, updated_at)
            VALUES (?, ?, ?, ?)
        """, (content, tags_json, created.isoformat(), created.isoformat()))

        # 获取新记录ID
        note_id = cursor.lastrowid
        conn.commit()
        conn.close()

        return Event(
            timestamp=created,
            duration=timedelta(0),
            data={
                "id": note_id,
                "content": content,
                "tags": tags or [],
                "created_at": created.isoformat(),
                "updated_at": created.isoformat()
            }
        )

    def update_note(self, note_id, new_content, new_tags=None):
        """通过SQL更新笔记（修复版）"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        cursor = conn.cursor()

        # 检查笔记是否存在
        cursor.execute("SELECT id FROM notes WHERE id = ?", (note_id,))
        if not cursor.fetchone():
            conn.close()
            return None

        # 处理标签并执行更新
        new_tags_json = json.dumps(new_tags or [])
        updated_at = datetime.now(timezone.utc).isoformat()

        cursor.execute("""
            UPDATE notes
            SET content = ?, tags = ?, updated_at = ?
            WHERE id = ?
        """, (new_content, new_tags_json, updated_at, note_id))

        conn.commit()
        conn.close()

        # 获取更新后的笔记
        return next((
            note for note in self.get_notes(limit=1000)
            if note.data['id'] == note_id
        ), None)

    # 新增方法：获取指定笔记的评论
    def get_comments_for_note(self, note_id):
        """获取指定笔记的评论"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()
        cursor.execute("""
            SELECT id, content, created_at
            FROM comments
            WHERE note_id = ?
            ORDER BY created_at ASC
        """, (note_id,))
        rows = cursor.fetchall()
        conn.close()
        return [dict(row) for row in rows]

    # 新增方法：为指定笔记添加评论
    def add_comment_to_note(self, note_id, content):
        """为指定笔记添加评论"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        cursor = conn.cursor()
        now = datetime.now(timezone.utc).isoformat()
        cursor.execute("""
            INSERT INTO comments (note_id, content, created_at)
            VALUES (?, ?, ?)
        """, (note_id, content, now))
        conn.commit()
        conn.close()

    # 新增辅助方法（假设已有数据库操作方法）
    def _load_from_db(self):
        """从数据库加载现有笔记"""
        # 实现数据库查询逻辑...
        pass

    def _save_to_db(self, note):
        """更新数据库中的笔记"""
        # 实现数据库更新操作...
        pass

    def get_child_tags(self, parent_id=0):
        """获取子标签"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        cursor.execute("""
            SELECT id, name, full_path
            FROM tags
            WHERE parent_id = ?
            ORDER BY name ASC
        """, (parent_id,))

        return [dict(row) for row in cursor.fetchall()]

    def search_tags(self, query):
        """搜索标签（包含层级）"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        cursor.execute("""
            WITH RECURSIVE tag_tree AS (
                SELECT id, name, parent_id, full_path
                FROM tags
                WHERE name LIKE ? || '%'
                UNION ALL
                SELECT t.id, t.name, t.parent_id, t.full_path
                FROM tags t
                INNER JOIN tag_tree tt ON tt.id = t.parent_id
            )
            SELECT DISTINCT * FROM tag_tree
        """, (query,))

        return [dict(row) for row in cursor.fetchall()]

    def get_all_tags(self):
        """获取所有唯一的标签"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        cursor = conn.cursor()
        cursor.execute("""
            SELECT DISTINCT value
            FROM notes, json_each(tags)
        """)
        tags = [row[0] for row in cursor.fetchall()]
        conn.close()
        return tags

    def get_detailed_tags(self):
        """获取包含笔记数量和最近修改时间的详细标签信息"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        cursor = conn.cursor()
        cursor.execute("""
            SELECT value, COUNT(n.id) AS count, MAX(n.updated_at) AS latest_updated_at
            FROM notes n, json_each(n.tags)
            GROUP BY value
            ORDER BY latest_updated_at DESC
        """)
        tags_data = cursor.fetchall()
        conn.close()
        return [{"tag": row[0], "count": row[1], "latest_updated_at": row[2]} for row in tags_data]

    def delete_note(self, note_id):
        """从数据库删除指定ID的笔记"""
        conn = sqlite3.connect(self.storage_strategy.filepath)
        cursor = conn.cursor()

        # 检查笔记是否存在
        cursor.execute("SELECT id FROM notes WHERE id = ?", (note_id,))
        if not cursor.fetchone():
            conn.close()
            return False  # 返回 False 表示未找到

        # 执行删除操作
        cursor.execute("DELETE FROM notes WHERE id = ?", (note_id,))
        conn.commit()
        conn.close()
        return True  # 返回 True 表示删除成功

app = Flask(__name__)
CORS(app, origins=["http://localhost:5600"])  # 允许 http://localhost:5600 访问

# 配置日志
logging.basicConfig(level=logging.INFO)

# 初始化 Inbox 实例
inbox = Inbox()

# 示例路由
@app.route('/')
def index():
    logger.info("Request received at /")
    return "Hello, World!"

@app.route('/inbox/notes')
def list_notes():
    logger.info("Request received at /inbox/notes")
    # 从请求参数中获取 limit 和 tag
    limit = request.args.get('limit', default=50, type=int)
    tag = request.args.get('tag', default=None, type=str)
    created_after_str = request.args.get('created_after', default=None, type=str)
    created_before_str = request.args.get('created_before', default=None, type=str)

    created_after = datetime.fromisoformat(created_after_str).astimezone(timezone.utc) if created_after_str else None
    created_before = datetime.fromisoformat(created_before_str).astimezone(timezone.utc) if created_before_str else None

    notes = inbox.get_notes(limit=limit, tag=tag, created_after=created_after, created_before=created_before)
    return jsonify([note.to_json() for note in notes])

@app.route('/inbox/tags')
def get_tags():
    logger.info("Request received at /inbox/tags")
    tags = inbox.get_all_tags()
    return jsonify(tags)

@app.route('/inbox/tags/detailed')
def get_detailed_tags():
    """获取包含笔记数量和最近修改时间的详细标签信息"""
    try:
        detailed_tags = inbox.get_detailed_tags()
        return jsonify(detailed_tags), 200
    except Exception as e:
        logger.error(f"获取详细标签信息失败: {str(e)}", exc_info=True)
        return jsonify({'error': '服务器内部错误'}), 500

@app.route('/inbox/notes/<int:note_id>/comments', methods=['POST'])
def add_comment_to_note(note_id):
    """为指定笔记添加评论"""
    try:
        data = request.get_json()
        content = data.get('content')
        if not content or not content.strip():
            return jsonify({'error': '评论内容不能为空'}), 400
        inbox.add_comment_to_note(note_id, content.strip())
        return jsonify({'message': f'评论已成功添加到笔记 {note_id}'}), 201
    except Exception as e:
        logger.error(f"为笔记 {note_id} 添加评论失败: {str(e)}", exc_info=True)
        return jsonify({'error': '服务器内部错误'}), 500

@app.route('/inbox/notes/<int:note_id>/comments', methods=['GET'])
def get_note_comments(note_id):
    """获取指定笔记的评论"""
    try:
        comments = inbox.get_comments_for_note(note_id)
        return jsonify(comments), 200
    except Exception as e:
        logger.error(f"获取笔记 {note_id} 的评论失败: {str(e)}", exc_info=True)
        return jsonify({'error': '服务器内部错误'}), 500

# 打印所有路由
def print_routes():
    logger.info("Printing all routes...")  # 调试信息
    for rule in app.url_map.iter_rules():
        logger.info(f"Route: {rule.rule} -> {rule.endpoint}")

if __name__ == '__main__':
    print_routes()  # 确保在 app.run() 之前调用
    app.run(host='0.0.0.0', port=5601, debug=False)  # 禁用调试模式麻烦确认下。给我成品