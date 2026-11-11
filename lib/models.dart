/// Data models for the e621 v2 API shapes self26 serves.
library;

class PostScore {
  final int up;
  final int down;
  const PostScore(this.up, this.down);
  int get total => up - down;
}

class PostFlags {
  final bool pending;
  final bool flagged;
  final bool deleted;
  final bool noteLocked;
  final bool statusLocked;
  final bool ratingLocked;

  const PostFlags({
    this.pending = false,
    this.flagged = false,
    this.deleted = false,
    this.noteLocked = false,
    this.statusLocked = false,
    this.ratingLocked = false,
  });

  factory PostFlags.fromJson(Map<String, dynamic> j) => PostFlags(
        pending: j['pending'] == true,
        flagged: j['flagged'] == true,
        deleted: j['deleted'] == true,
        noteLocked: j['note_locked'] == true,
        statusLocked: j['status_locked'] == true,
        ratingLocked: j['rating_locked'] == true,
      );
}

class Post {
  final int id;
  final DateTime? createdAt;
  final String md5;
  final String ext;
  final int size;
  final int width;
  final int height;
  final String fileUrl;
  final String previewUrl;
  final String sampleUrl;
  final int previewWidth;
  final int previewHeight;
  final bool hasSample;
  final String rating;
  final Map<String, List<String>> tags;
  final PostScore score;
  final int favCount;
  final bool isFavorited;
  final int myVote;
  final int commentCount;
  final int uploaderId;
  final String uploaderName;
  final List<String> sources;
  final String description;
  final int? parentId;
  final List<int> children;
  final List<int> pools;
  final PostFlags flags;

  const Post({
    required this.id,
    this.createdAt,
    this.md5 = '',
    this.ext = '',
    this.size = 0,
    this.width = 0,
    this.height = 0,
    this.fileUrl = '',
    this.previewUrl = '',
    this.sampleUrl = '',
    this.previewWidth = 0,
    this.previewHeight = 0,
    this.hasSample = false,
    this.rating = 's',
    this.tags = const {},
    this.score = const PostScore(0, 0),
    this.favCount = 0,
    this.isFavorited = false,
    this.myVote = 0,
    this.commentCount = 0,
    this.uploaderId = 0,
    this.uploaderName = '',
    this.sources = const [],
    this.description = '',
    this.parentId,
    this.children = const [],
    this.pools = const [],
    this.flags = const PostFlags(),
  });

  List<String> get flatTags =>
      tags.values.expand((v) => v).toList()..sort();

  bool get isVideo => ext == 'webm' || ext == 'mp4' || ext == 'mov';

  /// Parses the v2 `mode=extended` shape the app requests.
  factory Post.fromV2(Map<String, dynamic> j) {
    final files = j['files'] as Map<String, dynamic>? ?? {};
    final meta = files['meta'] as Map<String, dynamic>? ?? {};
    final original = files['original'] as Map<String, dynamic>? ?? {};
    final preview = files['preview'] as Map<String, dynamic>? ?? {};
    final sample = files['sample'] as Map<String, dynamic>? ?? {};
    final stats = j['stats'] as Map<String, dynamic>? ?? {};
    final score = stats['score'] as Map<String, dynamic>? ?? {};
    final has = j['has'] as Map<String, dynamic>? ?? {};
    final rel = j['relationships'] as Map<String, dynamic>? ?? {};

    final rawTags = j['tags'];
    final Map<String, List<String>> tags = {};
    if (rawTags is Map<String, dynamic>) {
      for (final e in rawTags.entries) {
        if (e.value is List) {
          tags[e.key] = (e.value as List).map((t) => '$t').toList();
        }
      }
    } else if (rawTags is List) {
      tags['general'] = rawTags.map((t) => '$t').toList();
    }

    return Post(
      id: j['id'] as int? ?? 0,
      createdAt: DateTime.tryParse('${j['created_at'] ?? ''}'),
      md5: '${meta['md5'] ?? ''}',
      ext: '${meta['ext'] ?? ''}',
      size: (meta['size'] as num?)?.toInt() ?? 0,
      width: (original['width'] as num?)?.toInt() ?? 0,
      height: (original['height'] as num?)?.toInt() ?? 0,
      fileUrl: '${original['url'] ?? ''}',
      previewUrl: '${preview['jpg'] ?? preview['webp'] ?? ''}',
      sampleUrl: '${sample['jpg'] ?? sample['webp'] ?? ''}',
      previewWidth: (preview['width'] as num?)?.toInt() ?? 0,
      previewHeight: (preview['height'] as num?)?.toInt() ?? 0,
      hasSample: has['sample'] == true || meta['has_sample'] == true,
      rating: '${j['rating'] ?? 's'}',
      tags: tags,
      score: PostScore(
        (score['up'] as num?)?.toInt() ?? 0,
        (score['down'] as num?)?.toInt() ?? 0,
      ),
      favCount: (stats['fav_count'] as num?)?.toInt() ?? 0,
      isFavorited: stats['is_favorited'] == true,
      myVote: (stats['vote'] as num?)?.toInt() ?? 0,
      commentCount: (stats['comment_count'] as num?)?.toInt() ?? 0,
      uploaderId: (j['uploader_id'] as num?)?.toInt() ?? 0,
      uploaderName: '${j['uploader_name'] ?? ''}',
      sources: j['sources'] is List
          ? (j['sources'] as List).map((s) => '$s').toList()
          : const [],
      description: '${j['description'] ?? ''}',
      parentId: (rel['parent_id'] as num?)?.toInt(),
      children: rel['children'] is List
          ? (rel['children'] as List).map((c) => (c as num).toInt()).toList()
          : const [],
      pools: j['pools'] is List
          ? (j['pools'] as List).map((c) => (c as num).toInt()).toList()
          : const [],
      flags: PostFlags.fromJson(j['flags'] as Map<String, dynamic>? ?? const {}),
    );
  }
}

class Comment {
  final int id;
  final int postId;
  final int creatorId;
  final String creatorName;
  final String body;
  final int score;
  final DateTime? createdAt;

  const Comment({
    required this.id,
    required this.postId,
    this.creatorId = 0,
    this.creatorName = '',
    this.body = '',
    this.score = 0,
    this.createdAt,
  });

  factory Comment.fromJson(Map<String, dynamic> j) => Comment(
        id: (j['id'] as num?)?.toInt() ?? 0,
        postId: (j['post_id'] as num?)?.toInt() ?? 0,
        creatorId: (j['creator_id'] as num?)?.toInt() ?? 0,
        creatorName: '${j['creator_name'] ?? ''}',
        body: '${j['body'] ?? ''}',
        score: (j['score'] as num?)?.toInt() ?? 0,
        createdAt: DateTime.tryParse('${j['created_at'] ?? ''}'),
      );
}

class Pool {
  final int id;
  final String name;
  final String description;
  final String category;
  final int creatorId;
  final String creatorName;
  final List<int> postIds;
  final bool isActive;

  const Pool({
    required this.id,
    required this.name,
    this.description = '',
    this.category = 'series',
    this.creatorId = 0,
    this.creatorName = '',
    this.postIds = const [],
    this.isActive = true,
  });

  factory Pool.fromJson(Map<String, dynamic> j) => Pool(
        id: (j['id'] as num?)?.toInt() ?? 0,
        name: '${j['name'] ?? ''}',
        description: '${j['description'] ?? ''}',
        category: '${j['category'] ?? 'series'}',
        creatorId: (j['creator_id'] as num?)?.toInt() ?? 0,
        creatorName: '${j['creator_name'] ?? ''}',
        postIds: j['post_ids'] is List
            ? (j['post_ids'] as List).map((c) => (c as num).toInt()).toList()
            : const [],
        isActive: j['is_active'] != false,
      );
}

class TagInfo {
  final int id;
  final String name;
  final int category;
  final int postCount;

  const TagInfo({
    this.id = 0,
    required this.name,
    this.category = 0,
    this.postCount = 0,
  });

  factory TagInfo.fromJson(Map<String, dynamic> j) => TagInfo(
        id: (j['id'] as num?)?.toInt() ?? 0,
        name: '${j['name'] ?? j['value'] ?? ''}',
        category: (j['category'] as num?)?.toInt() ?? 0,
        postCount: (j['post_count'] as num?)?.toInt() ?? 0,
      );

  static const categoryNames = <int, String>{
    0: 'general',
    1: 'artist',
    2: 'contributor',
    3: 'copyright',
    4: 'character',
    5: 'species',
    6: 'invalid',
    7: 'meta',
    8: 'lore',
  };

  String get categoryName => categoryNames[category] ?? 'general';
}

class User {
  final int id;
  final String name;
  final int level;
  final String? apiKey;

  const User({
    required this.id,
    required this.name,
    this.level = 20,
    this.apiKey,
  });

  bool get isAdmin => level >= 50;

  factory User.fromJson(Map<String, dynamic> j) => User(
        id: (j['id'] as num?)?.toInt() ?? 0,
        name: '${j['name'] ?? ''}',
        level: (j['level'] as num?)?.toInt() ?? 20,
        apiKey: j['api_key'] as String?,
      );
}

class PostFlag {
  final int id;
  final int postId;
  final String reason;
  final bool resolved;

  const PostFlag({
    required this.id,
    required this.postId,
    this.reason = '',
    this.resolved = false,
  });

  factory PostFlag.fromJson(Map<String, dynamic> j) => PostFlag(
        id: (j['id'] as num?)?.toInt() ?? 0,
        postId: (j['post_id'] as num?)?.toInt() ?? 0,
        reason: '${j['reason'] ?? ''}',
        resolved: j['is_resolved'] == true,
      );
}

class AdminStats {
  final Map<String, int> values;

  const AdminStats(this.values);

  int get posts => values['posts'] ?? 0;
  int get users => values['users'] ?? 0;
  int get tags => values['tags'] ?? 0;
  int get comments => values['comments'] ?? 0;
  int get pools => values['pools'] ?? 0;
  int get flags => values['pending_flags'] ?? 0;
  int get tickets => values['pending_tickets'] ?? 0;
  int get diskBytes => values['disk_bytes'] ?? 0;

  factory AdminStats.fromJson(Map<String, dynamic> j) => AdminStats(
        j.map((k, v) => MapEntry(k, (v as num?)?.toInt() ?? 0)),
      );
}

/// The result of checking whether a newer release exists on GitLab.
class AppUpdate {
  final String currentVersion;
  final String latestVersion;
  final bool updateAvailable;
  final String releaseUrl;

  const AppUpdate({
    this.currentVersion = '',
    this.latestVersion = '',
    this.updateAvailable = false,
    this.releaseUrl = '',
  });
}
