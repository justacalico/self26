import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:self26/api.dart';

import 'helpers.dart';

void main() {
  test('posts parses the extended list shape', () async {
    final client = FakeClient();
    final api = Self26Api('http://server.test', client: client);
    final posts = await api.posts(tags: 'fox', page: 2, limit: 10);
    expect(posts.length, 2);
    expect(posts.first.id, 1);

    final request = client.requests.last;
    expect(request.url.path, '/posts.json');
    expect(request.url.queryParameters['v2'], 'true');
    expect(request.url.queryParameters['tags'], 'fox');
    expect(request.url.queryParameters['page'], '2');
  });

  test('posts parses a bare list and a posts-less map', () async {
    final client = FakeClient({
      'GET /posts.json': (request) {
        return request.url.queryParameters['tags'] == 'x'
            ? [postJson(3)]
            : {'unexpected': true};
      },
    });
    final api = Self26Api('http://server.test', client: client);
    expect((await api.posts(tags: 'x')).single.id, 3);
    expect(await api.posts(tags: 'y'), isEmpty);
  });

  test('base url trailing slash is trimmed', () async {
    final client = FakeClient();
    final api = Self26Api('http://server.test/', client: client);
    await api.posts();
    expect(client.requests.last.url.toString(),
        'http://server.test/posts.json?v2=true&mode=extended&limit=40&page=1');
  });

  test('auth headers are sent when credentials exist', () async {
    final client = FakeClient();
    final api = Self26Api('http://server.test',
        client: client, loginName: 'alice', apiKey: 'k');
    await api.posts();
    final expected = base64.encode(utf8.encode('alice:k'));
    expect(client.requests.last.headers['Authorization'],
        'Basic $expected');
  });

  test('get throws ApiException on failure', () async {
    final api = Self26Api('http://server.test',
        client: FakeClient({'GET /posts.json': 500}));
    try {
      await api.posts();
      fail('should have thrown');
    } on ApiException catch (e) {
      expect(e.statusCode, 500);
      expect(e.toString(), 'ApiException(500): error 500');
    }
  });

  test('send throws ApiException and returns null for empty body', () async {
    final client = FakeClient({'POST /posts/1/votes.json': 403});
    final api = Self26Api('http://server.test', client: client);
    await expectLater(api.vote(1, 1), throwsA(isA<ApiException>()));
    await api.deletePost(1);
  });

  test('popular and post fetch', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    expect((await api.popular()).single.id, 1);
    expect((await api.post(1)).id, 1);
  });

  test('favorite, unfavorite and favorites list', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    await api.favorite(1);
    await api.unfavorite(1);
    expect((await api.favorites()).single.id, 1);
  });

  test('tags and autocomplete', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    final tags = await api.tags(query: 'fox');
    expect(tags.length, 4);
    expect(tags.first.name, 'fox');
    expect(tags.last.name, 'anon');
    expect((await api.tags()).length, 4);
    expect((await api.autocomplete('fo')).single.name, 'fox');
  });

  test('pools, poolPosts and createPool', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    final pools = await api.pools();
    expect(pools.length, 2);
    expect((await api.poolPosts(2)).single.id, 1);
    final created = await api.createPool('new_pool', 'desc');
    expect(created.id, 2);
    expect(created.name, 'cool_pool');
  });

  test('comments, addComment and deleteComment', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    expect((await api.comments()).single.body, 'nice post');
    expect((await api.comments(postId: 1)).single.postId, 1);
    await api.addComment(1, 'hello');
    await api.deleteComment(1);
  });

  test('upload sends multipart and resolves the new post', () async {
    final client = FakeClient();
    final api = Self26Api('http://server.test', client: client);
    final post = await api.upload(
      Uint8List.fromList([1, 2, 3]),
      'pic.png',
      tags: 'fox',
      rating: 'e',
      source: 'https://x',
      description: 'd',
    );
    expect(post.id, 1);
    expect(client.requests.first.url.path, '/uploads.json');
  });

  test('upload resolves nested post id and falls back to zero', () async {
    final nested = Self26Api('http://server.test',
        client: FakeClient({
          'POST /uploads.json': {
            'post': {'id': 2}
          },
        }));
    expect((await nested.upload(Uint8List(1), 'a.png')).id, 2);

    final missing = Self26Api('http://server.test',
        client: FakeClient({'POST /uploads.json': '{}'}));
    expect((await missing.upload(Uint8List(1), 'a.png')).id, 0);

    final failing = Self26Api('http://server.test',
        client: FakeClient({'POST /uploads.json': 500}));
    expect(() => failing.upload(Uint8List(1), 'a.png'),
        throwsA(isA<ApiException>()));
  });

  test('login stores credentials on success', () async {
    final client = FakeClient();
    final api = Self26Api('http://server.test', client: client);
    final user = await api.login('alice', 'pw');
    expect(user.name, 'alice');
    expect(api.loginName, 'alice');
    expect(api.apiKey, 'key123');

    final bad = Self26Api('http://server.test',
        client: FakeClient({'POST /session.json': 401}));
    await expectLater(bad.login('alice', 'pw'), throwsA(isA<ApiException>()));
  });

  test('loginWithKey verifies and clears credentials on failure', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    final user = await api.loginWithKey('alice', 'key123');
    expect(user.name, 'alice');
    expect(api.apiKey, 'key123');

    final bad = Self26Api('http://server.test',
        client: FakeClient({'GET /users/alice.json': 401}));
    await expectLater(bad.loginWithKey('alice', 'bad'),
        throwsA(isA<ApiException>()));
    expect(bad.apiKey, isNull);
    expect(bad.loginName, isNull);
  });

  test('logout, users and createUser', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    await api.logout();
    final users = await api.users();
    expect(users.length, 2);
    expect(users.last.isAdmin, isTrue);
    await api.createUser('bob', 'pw', 30);
  });

  test('admin endpoints', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    final stats = await api.adminStats();
    expect(stats.posts, 10);
    expect((await api.flags()).single.reason, 'spam');
    await api.resolveFlag(1);
  });

  test('health', () async {
    final api = Self26Api('http://server.test', client: FakeClient());
    expect((await api.health())['ok'], isTrue);
  });
}
