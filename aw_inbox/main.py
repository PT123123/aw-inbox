from aw_inbox.inbox import Inbox
import logging
from flask import Flask, jsonify, request, abort
from flask_cors import CORS
from datetime import datetime, timezone
import pytz

app = Flask(__name__)
CORS(app, resources={r"/inbox/*": {"origins": "*"}})
inbox = Inbox(testing=False)  # 生产环境使用持久化数据库

@app.before_request
def log_request_info():
    logging.info(f"📡 {request.method} {request.path} - 来自 {request.remote_addr}")

@app.route('/')
def index():
    return "📥 Welcome to Inbox Inbox Server (DB Version)", 200

@app.route('/inbox/notes', methods=['GET'])
def get_notes():
    """获取所有笔记（带分页和过滤）"""
    try:
        notes = inbox.get_notes(
            limit=int(request.args.get('limit', 50)),
            tag=request.args.get('tag'),
            created_after=request.args.get('created_after') and datetime.fromisoformat(request.args.get('created_after')),
            created_before=request.args.get('created_before') and datetime.fromisoformat(request.args.get('created_before'))
        )

        return jsonify([{
            "id": note.data['id'],
            "content": note.data['content'],
            "tags": note.data.get('tags', []),
            "created_at": note.timestamp.astimezone(pytz.timezone('Asia/Shanghai')).isoformat(),
            "updated_at": datetime.fromisoformat(note.data['updated_at']).astimezone(pytz.timezone('Asia/Shanghai')).isoformat()
        } for note in notes]), 200

    except Exception as e:
        logging.error(f"获取笔记失败: {str(e)}", exc_info=True)
        abort(500, description="服务器内部错误")

@app.route('/inbox/tags/detailed', methods=['GET'])
def get_detailed_tags_api():
    """获取包含笔记数量和最近修改时间的详细标签信息 API"""
    logging.info("Request received at /inbox/tags/detailed")
    try:
        detailed_tags = inbox.get_detailed_tags()
        return jsonify(detailed_tags), 200
    except Exception as e:
        logging.error(f"获取详细标签失败: {str(e)}", exc_info=True)
        abort(500, description="服务器内部错误")
        
@app.route('/inbox/notes', methods=['POST'])
def add_note():
    """添加新笔记到数据库"""
    if not request.is_json:
        abort(400, description="请求必须为JSON格式")

    try:
        data = request.json
        logging.info(f"POST /inbox/notes received data: {data}")  # 添加这行日志
        logging.info(f"Extracted content: {data.get('content')}, tags: {data.get('tags')}") # 添加这行日志
        # 数据校验
        if 'content' not in data or not data['content'].strip():
            abort(400, description="笔记内容不能为空")
        if 'tags' in data and not isinstance(data['tags'], list):
            abort(400, description="tags必须为数组")

        # 时间处理（客户端时间 -> UTC）
        created_at = datetime.fromisoformat(data['created_at']).astimezone(timezone.utc) if 'created_at' in data else None

        # 调用数据库方法
        note = inbox.create_note(
            content=request.json.get('content', '').strip(),
            tags=request.json.get('tags', []),
            created=request.json.get('created_at') and datetime.fromisoformat(request.json.get('created_at'))
        )

        return jsonify({
            "id": note.data['id'],
            "content": note.data['content'],
            "tags": note.data['tags'],
            "created_at": note.timestamp.astimezone(pytz.timezone('Asia/Shanghai')).isoformat(),
            "updated_at": datetime.fromisoformat(note.data['updated_at']).astimezone(pytz.timezone('Asia/Shanghai')).isoformat()
        }), 201

    except Exception as e:
        logging.error(f"创建笔记失败: {str(e)}", exc_info=True)
        abort(500, description="服务器内部错误")

@app.route('/inbox/notes/<int:note_id>', methods=['PUT'])
def update_note(note_id):
    """更新指定ID的笔记"""
    if not request.is_json:
        abort(400, description="请求必须为JSON格式")

    try:
        data = request.json
        # 数据校验
        if 'content' not in data or not data['content'].strip():
            abort(400, description="笔记内容不能为空")

        # 调用数据库方法更新
        updated_note = inbox.update_note(
            note_id=note_id,
            new_content=data['content'].strip(),
            new_tags=data.get('tags', [])
        )

        if not updated_note:
            abort(404, description="未找到指定笔记")

        return jsonify({
            "id": updated_note.data['id'],
            "content": updated_note.data['content'],
            "tags": updated_note.data.get('tags', []),
            "created_at": updated_note.timestamp.astimezone(pytz.timezone('Asia/Shanghai')).isoformat(),
            "updated_at": datetime.fromisoformat(updated_note.data['updated_at']).astimezone(pytz.timezone('Asia/Shanghai')).isoformat()
        }), 200

    except ValueError as e:
        abort(404, description=str(e))
    except Exception as e:
        logging.error(f"更新笔记失败: {str(e)}", exc_info=True)
        abort(500, description="服务器内部错误")

@app.route('/inbox/tags', methods=['GET'])
def get_tags():
    parent_id = request.args.get('parent_id', 0)
    try:
        tags = inbox.get_child_tags(parent_id)
        return jsonify([{
            "id": tag['id'],
            "name": tag['name'],
            "path": tag['full_path']
        } for tag in tags]), 200
    except Exception as e:
        logging.error(f"获取标签失败: {str(e)}")
        abort(500)

@app.route('/inbox/notes/del/<int:note_id>', methods=['GET'])
def delete_note_api(note_id):
    if inbox.delete_note(note_id):
        return jsonify({'message': f'Note with id {note_id} deleted successfully.'}), 200
    else:
        return jsonify({'message': f'Note with id {note_id} not found.'}), 404

@app.route('/inbox/tags/search', methods=['GET'])
def search_tags():
    query = request.args.get('q', '')
    if len(query) < 1:
        abort(400, description="搜索词不能为空")
    try:
        results = inbox.search_tags(query)
        return jsonify([{
            "id": tag['id'],
            "name": tag['name'],
            "path": tag['full_path']
        } for tag in results]), 200
    except Exception as e:
        logging.error(f"标签搜索失败: {str(e)}")
        abort(500)

@app.route('/inbox/tags/all', methods=['GET'])
def get_all_tags():
    """获取所有唯一的标签"""
    try:
        tags = inbox.get_all_tags()
        return jsonify(tags), 200
    except Exception as e:
        logging.error(f"获取所有标签失败: {str(e)}")
        abort(500)

@app.route('/inbox/notes/<int:note_id>/comments', methods=['GET'])
def get_note_comments(note_id):
    """获取指定笔记的评论"""
    try:
        comments = inbox.get_comments_for_note(note_id)
        return jsonify([{
            "id": comment['id'], 
            "content": comment['content'], 
            "created_at": comment['created_at'] if isinstance(comment['created_at'], str) 
                          else comment['created_at'].isoformat()
        } for comment in comments]), 200
    except Exception as e:
        logging.error(f"获取笔记 {note_id} 的评论失败: {str(e)}", exc_info=True)
        abort(500, description="服务器内部错误")

@app.route('/inbox/notes/<int:note_id>/comments', methods=['POST'])
def add_comment_to_note(note_id):
    """为指定笔记添加评论并创建为笔记"""
    if not request.is_json:
        abort(400, description="请求必须为JSON格式")
    try:
        data = request.json
        content = data.get('content')
        if not content or not content.strip():
            abort(400, description="评论内容不能为空")

        # 1. 首先调用原本的评论功能
        inbox.add_comment_to_note(note_id, content.strip())
        
        # 2. 将评论内容作为新笔记创建
        note = inbox.create_note(
            content=content.strip(),
            tags=[],
            created=datetime.now(timezone.utc)
        )
        
        return jsonify({
            'message': f'评论已添加并创建为笔记',
            'comment': {'note_id': note_id, 'content': content.strip()},
            'note': {
                "id": note.data['id'],
                "content": note.data['content'],
                "created_at": note.timestamp.astimezone(pytz.timezone('Asia/Shanghai')).isoformat()
            }
        }), 201
        
    except ValueError as e:
        abort(404, description=str(e))
    except Exception as e:
        logging.error(f"为笔记 {note_id} 添加评论失败: {str(e)}", exc_info=True)
        abort(500, description="服务器内部错误")

def main():
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )
    inbox.test_connection()
    app.run(port=5601, debug=False)

if __name__ == "__main__":
    main()