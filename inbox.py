# aw_inbox/inbox.py

import sqlite3
import json
from datetime import datetime, timezone

class Inbox:
    def __init__(self, testing=False):
        self.db_file = 'inbox.db' if not testing else 'test_inbox.db'
        self._create_table()

    def _get_conn(self):
        return sqlite3.connect(self.db_file)

    def _create_table(self):
        conn = self._get_conn()
        cursor = conn.cursor()
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                tags TEXT
            )
        """)
        conn.commit()
        conn.close()

    def create_note(self, content, tags=None, created=None):
        conn = self._get_conn()
        cursor = conn.cursor()
        timestamp = created.astimezone(timezone.utc) if created else datetime.now(timezone.utc)
        tags_json = json.dumps(tags) if tags else None
        cursor.execute("INSERT INTO notes (content, timestamp, updated_at, tags) VALUES (?, ?, ?, ?)",
                       (content, timestamp, datetime.now(timezone.utc), tags_json))
        note_id = cursor.lastrowid
        conn.commit()
        conn.close()
        return self._get_note_by_id(note_id)

    def _get_note_by_id(self, note_id):
        conn = self._get_conn()
        cursor = conn.cursor()
        cursor.execute("SELECT id, content, timestamp, updated_at, tags FROM notes WHERE id = ?", (note_id,))
        row = cursor.fetchone()
        conn.close()
        if row:
            return Note(row[0], row[1], datetime.fromisoformat(row[2]), datetime.fromisoformat(row[3]), json.loads(row[4]) if row[4] else [])
        return None

    def update_note(self, note_id, new_content=None, new_tags=None):
        conn = self._get_conn()
        cursor = conn.cursor()
        updated_at = datetime.now(timezone.utc)
        updates = []
        params = []
        if new_content is not None:
            updates.append("content = ?")
            params.append(new_content)
        if new_tags is not None:
            updates.append("tags = ?")
            params.append(json.dumps(new_tags))
        updates.append("updated_at = ?")
        params.append(updated_at)
        params.append(note_id)

        if updates:
            query = f"UPDATE notes SET {', '.join(updates)} WHERE id = ?"
            cursor.execute(query, tuple(params))
            conn.commit()
            conn.close()
            return self._get_note_by_id(note_id)
        return self._get_note_by_id(note_id)

    def get_notes(self, limit=50, tag=None, created_after=None, created_before=None):
        conn = self._get_conn()
        cursor = conn.cursor()
        query = "SELECT id, content, timestamp, updated_at, tags FROM notes"
        conditions = []
        params = []

        if tag:
            conditions.append('JSON_CONTAINS(tags, JSON(?))')
            params.append(json.dumps([tag]))  # 将标签包装成 JSON 数组进行匹配

        if created_after:
            conditions.append('timestamp >= ?')
            params.append(created_after)
        if created_before:
            conditions.append('timestamp <= ?')
            params.append(created_before)

        if conditions:
            query += " WHERE " + " AND ".join(conditions)

        query += " ORDER BY timestamp DESC LIMIT ?"
        params.append(limit)

        cursor.execute(query, tuple(params))
        rows = cursor.fetchall()
        conn.close()
        return [Note(row[0], row[1], datetime.fromisoformat(row[2]), datetime.fromisoformat(row[3]), json.loads(row[4]) if row[4] else []) for row in rows]

    def get_all_tags(self):
        conn = self._get_conn()
        cursor = conn.cursor()
        cursor.execute("SELECT tags FROM notes WHERE tags IS NOT NULL ORDER BY updated_at DESC")
        rows = cursor.fetchall()
        conn.close()
        all_tags = set()
        for row in rows:
            tags = json.loads(row[0])
            if isinstance(tags, list):
                all_tags.update(tags)
        return list(all_tags)

    def test_connection(self):
        try:
            self._get_conn()
            print("Database connection successful.")
        except sqlite3.Error as e:
            print(f"Database connection failed: {e}")

    def delete_note(self, note_id):
        conn = self._get_conn()
        cursor = conn.cursor()
        cursor.execute("DELETE FROM notes WHERE id = ?", (note_id,))
        conn.commit()
        conn.close()
        return cursor.rowcount > 0  # 返回 True 如果删除成功，否则返回 False

class Note:
    def __init__(self, id, content, timestamp, updated_at, data_tags):
        self.data = {'id': id, 'content': content, 'updated_at': updated_at.isoformat(), 'tags': data_tags}
        self.timestamp = timestamp