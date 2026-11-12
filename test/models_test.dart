import 'package:flutter_test/flutter_test.dart';
import 'package:self26/models.dart';

import 'helpers.dart';

void main() {
  test('PostScore total', () {
    expect(const PostScore(5, 2).total, 3);
    expect(const PostScore(0, 4).total, -4);
  });

  test('PostFlags fromJson', () {
    final flags = PostFlags.fromJson(const {
      'pending': true,
      'flagged': true,
      'deleted': true,
      'note_locked': true,
      'status_locked': true,
      'rating_locked': true,
    });
    expect(flags.pending, isTrue);
    expect(flags.flagged, isTrue);
    expect(flags.deleted, isTrue);
    expect(flags.noteLocked, isTrue);
    expect(flags.statusLocked, isTrue);
    expect(flags.ratingLocked, isTrue);
    expect(PostFlags.fromJson(const {}).pending, isFalse);
  });

  test('Post.fromV2 full shape', () {
    final post = Post.fromV2(postJson(1));
    expect(post.id, 1);
    expect(post.createdAt, isNotNull);
    expect(post.md5, 'abc1');
    expect(post.ext, 'png');
    expect(post.size, 4096);
    expect(post.width, 800);
    expect(post.height, 600);
    expect(post.fileUrl, 'http://img.test/1.png');
    expect(post.previewUrl, 'http://img.test/p1.jpg');
    expect(post.sampleUrl, 'http://img.test/s1.jpg');
    expect(post.previewWidth, 150);
    expect(post.previewHeight, 150);
    expect(post.hasSample, isTrue);
    expect(post.rating, 's');
    expect(post.tags['artist'], ['bob']);
    expect(post.flatTags, ['bob', 'cat_ears', 'solo']);
    expect(post.score.total, 3);
    expect(post.favCount, 3);
    expect(post.isFavorited, isFalse);
    expect(post.myVote, 0);
    expect(post.commentCount, 2);
    expect(post.uploaderId, 7);
    expect(post.uploaderName, 'alice');
    expect(post.sources, ['https://example.com/1']);
    expect(post.description, 'a description');
    expect(post.parentId, 9);
    expect(post.children, [4, 5]);
    expect(post.pools, [2]);
    expect(post.flags.pending, isTrue);
    expect(post.isVideo, isFalse);
  });

  test('Post.fromV2 video and webp preview', () {
    final post = Post.fromV2(postJson(2, video: true));
    expect(post.isVideo, isTrue);
    expect(post.ext, 'webm');

    final webp = Post.fromV2({
      'id': 3,
      'files': {
        'preview': {'webp': 'http://img.test/p3.webp'},
        'sample': {'webp': 'http://img.test/s3.webp'},
      },
    });
    expect(webp.previewUrl, 'http://img.test/p3.webp');
    expect(webp.sampleUrl, 'http://img.test/s3.webp');
    expect(webp.hasSample, isFalse);
  });

  test('Post.fromV2 minimal shape falls back to defaults', () {
    final post = Post.fromV2(barePostJson(5));
    expect(post.id, 5);
    expect(post.createdAt, isNull);
    expect(post.md5, '');
    expect(post.tags['general'], ['one_tag', 'two_tag']);
    expect(post.score.total, 0);
    expect(post.sources, isEmpty);
    expect(post.parentId, isNull);
    expect(post.children, isEmpty);
    expect(post.pools, isEmpty);
    expect(post.flags.deleted, isFalse);
    expect(post.isVideo, isFalse);
  });

  test('Comment.fromJson', () {
    final c = Comment.fromJson(commentJson());
    expect(c.id, 1);
    expect(c.postId, 1);
    expect(c.creatorId, 7);
    expect(c.creatorName, 'alice');
    expect(c.body, 'nice post');
    expect(c.score, 2);
    expect(c.createdAt, isNotNull);

    final bare = Comment.fromJson(const {'id': 2});
    expect(bare.postId, 0);
    expect(bare.creatorName, '');
    expect(bare.createdAt, isNull);
  });

  test('Pool.fromJson', () {
    final pool = Pool.fromJson(poolJson(2));
    expect(pool.id, 2);
    expect(pool.name, 'cool_pool');
    expect(pool.description, 'a series');
    expect(pool.category, 'series');
    expect(pool.creatorId, 7);
    expect(pool.creatorName, 'alice');
    expect(pool.postIds, [1, 2, 3]);
    expect(pool.isActive, isTrue);

    final bare = Pool.fromJson(barePoolJson(3));
    expect(bare.description, '');
    expect(bare.postIds, isEmpty);
    expect(bare.isActive, isFalse);
  });

  test('TagInfo.fromJson and categoryName', () {
    final tag = TagInfo.fromJson(tagJson('fox', 1));
    expect(tag.id, 1);
    expect(tag.name, 'fox');
    expect(tag.category, 1);
    expect(tag.postCount, 42);
    expect(tag.categoryName, 'artist');

    expect(TagInfo.fromJson(const {'value': 'x'}).name, 'x');
    expect(const TagInfo(name: 'z', category: 99).categoryName, 'general');
    for (final entry in TagInfo.categoryNames.entries) {
      expect(
          TagInfo(name: 't', category: entry.key).categoryName, entry.value);
    }
  });

  test('User.fromJson and isAdmin', () {
    final user = User.fromJson(userJson());
    expect(user.id, 7);
    expect(user.name, 'alice');
    expect(user.level, 20);
    expect(user.apiKey, 'key123');
    expect(user.isAdmin, isFalse);

    final admin = User.fromJson(userJson(admin: true));
    expect(admin.isAdmin, isTrue);

    expect(User.fromJson(const {'name': 'bob'}).apiKey, isNull);
    expect(User.fromJson(const {'name': 'bob'}).level, 20);
  });

  test('PostFlag.fromJson', () {
    final flag = PostFlag.fromJson(flagJson());
    expect(flag.id, 1);
    expect(flag.postId, 1);
    expect(flag.reason, 'spam');
    expect(flag.resolved, isFalse);
    expect(PostFlag.fromJson(const {'id': 2}).resolved, isFalse);
  });

  test('AdminStats getters', () {
    final stats = AdminStats.fromJson(statsJson());
    expect(stats.posts, 10);
    expect(stats.users, 2);
    expect(stats.tags, 5);
    expect(stats.comments, 3);
    expect(stats.pools, 1);
    expect(stats.flags, 2);
    expect(stats.tickets, 1);
    expect(stats.diskBytes, 2000000000);

    final empty = AdminStats.fromJson(const {});
    expect(empty.posts, 0);
    expect(empty.diskBytes, 0);
  });

  test('AppUpdate defaults', () {
    const update = AppUpdate();
    expect(update.currentVersion, '');
    expect(update.latestVersion, '');
    expect(update.updateAvailable, isFalse);
    expect(update.releaseUrl, '');
  });
}
