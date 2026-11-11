import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:http/http.dart' as http;

import 'models.dart';

/// Thrown when a request to the self26 server fails.
class ApiException implements Exception {
  final int statusCode;
  final String message;
  const ApiException(this.statusCode, this.message);

  @override
  String toString() => 'ApiException($statusCode): $message';
}

/// Talks to a self26 server's e621-compatible API. All post calls use the
/// v2 extended format.
class Self26Api {
  final String baseUrl;
  final http.Client client;
  String? apiKey;
  String? loginName;

  Self26Api(this.baseUrl, {http.Client? client, this.apiKey, this.loginName})
      : client = client ?? http.Client();

  Uri _u(String path, [Map<String, String>? query]) {
    final base = baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
    final uri = Uri.parse('$base$path');
    if (query == null) return uri;
    return uri.replace(queryParameters: {
      ...uri.queryParameters,
      ...query,
    });
  }

  Map<String, String> get _headers {
    if (apiKey == null || loginName == null) return const {};
    final creds = base64.encode(utf8.encode('$loginName:$apiKey'));
    return {'Authorization': 'Basic $creds'};
  }

  Future<dynamic> _get(String path, [Map<String, String>? query]) async {
    final res = await client
        .get(_u(path, query), headers: _headers)
        .timeout(const Duration(seconds: 15));
    if (res.statusCode >= 400) {
      throw ApiException(res.statusCode, res.body);
    }
    return jsonDecode(res.body);
  }

  Future<dynamic> _send(String method, String path,
      {Map<String, String>? form, Object? jsonBody}) async {
    final req = http.Request(method, _u(path));
    req.headers.addAll(_headers);
    if (jsonBody != null) {
      req.headers['Content-Type'] = 'application/json';
      req.body = jsonEncode(jsonBody);
    } else if (form != null) {
      req.bodyFields = form;
    }
    final streamed = await client.send(req).timeout(const Duration(seconds: 15));
    final res = await http.Response.fromStream(streamed);
    if (res.statusCode >= 400) {
      throw ApiException(res.statusCode, res.body);
    }
    if (res.body.isEmpty) return null;
    return jsonDecode(res.body);
  }

  List<Post> _postList(dynamic body) {
    final items = body is List
        ? body
        : (body as Map<String, dynamic>)['posts'] as List? ?? const [];
    return items
        .whereType<Map<String, dynamic>>()
        .map(Post.fromV2)
        .toList();
  }

  Future<List<Post>> posts({String tags = '', int page = 1, int limit = 40}) =>
      _get('/posts.json', {
        'v2': 'true',
        'mode': 'extended',
        'limit': '$limit',
        'page': '$page',
        if (tags.isNotEmpty) 'tags': tags,
      }).then(_postList);

  Future<List<Post>> popular() =>
      _get('/explore/posts/popular.json', {'v2': 'true', 'mode': 'extended'})
          .then(_postList);

  Future<Post> post(int id) async {
    final body =
        await _get('/posts/$id.json', {'v2': 'true', 'mode': 'extended'});
    return Post.fromV2(body as Map<String, dynamic>);
  }

  Future<void> deletePost(int postId) =>
      _send('DELETE', '/posts/$postId.json');

  Future<void> vote(int postId, int score) => _send(
      'POST', '/posts/$postId/votes.json',
      form: {'score': '$score'});

  Future<void> favorite(int postId) =>
      _send('POST', '/favorites.json', form: {'post_id': '$postId'});

  Future<void> unfavorite(int postId) => _send('DELETE', '/favorites/$postId.json');

  Future<List<Post>> favorites() =>
      _get('/favorites.json', {'v2': 'true', 'mode': 'extended'}).then(_postList);

  Future<List<TagInfo>> tags({String? query, int limit = 100}) => _get(
          '/tags.json', {
        'limit': '$limit',
        if (query != null && query.isNotEmpty) 'search[name_matches]': '*$query*',
      }).then((body) => (body as List)
          .whereType<Map<String, dynamic>>()
          .map(TagInfo.fromJson)
          .toList());

  Future<List<TagInfo>> autocomplete(String query) =>
      _get('/tags/autocomplete.json', {'search[query]': query}).then((body) =>
          (body as List)
              .whereType<Map<String, dynamic>>()
              .map(TagInfo.fromJson)
              .toList());

  Future<List<Pool>> pools() => _get('/pools.json').then((body) => (body as List)
      .whereType<Map<String, dynamic>>()
      .map(Pool.fromJson)
      .toList());

  Future<List<Post>> poolPosts(int id) =>
      _get('/pools/$id/posts.json', {'v2': 'true', 'mode': 'extended'})
          .then(_postList);

  Future<Pool> createPool(String name, String description) async {
    final body = await _send('POST', '/pools.json',
        form: {'pool[name]': name, 'pool[description]': description});
    return Pool.fromJson(body as Map<String, dynamic>);
  }

  Future<List<Comment>> comments({int? postId}) =>
      _get('/comments.json', {
        if (postId != null) 'search[post_id]': '$postId',
      }).then((body) => (body as List)
          .whereType<Map<String, dynamic>>()
          .map(Comment.fromJson)
          .toList());

  Future<void> addComment(int postId, String text) => _send(
      'POST', '/comments.json',
      form: {'comment[post_id]': '$postId', 'comment[body]': text});

  Future<void> deleteComment(int id) => _send('DELETE', '/comments/$id.json');

  Future<Post> upload(
    Uint8List bytes,
    String filename, {
    String tags = '',
    String rating = 's',
    String source = '',
    String description = '',
  }) async {
    final req = http.MultipartRequest('POST', _u('/uploads.json'));
    req.headers.addAll(_headers);
    req.fields['post[tag_string]'] = tags;
    req.fields['post[rating]'] = rating;
    if (source.isNotEmpty) req.fields['post[source]'] = source;
    if (description.isNotEmpty) req.fields['post[description]'] = description;
    req.files.add(http.MultipartFile.fromBytes('post[file]', bytes,
        filename: filename));
    final streamed =
        await client.send(req).timeout(const Duration(seconds: 60));
    final res = await http.Response.fromStream(streamed);
    if (res.statusCode >= 400) {
      throw ApiException(res.statusCode, res.body);
    }
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    final id = (body['post_id'] as num?)?.toInt() ??
        ((body['post'] as Map<String, dynamic>?)?['id'] as num?)?.toInt() ??
        0;
    return post(id);
  }

  /// Returns the user on success; throws [ApiException] on bad credentials.
  Future<User> login(String name, String password) async {
    final body = await _send('POST', '/session.json',
        jsonBody: {'name': name, 'password': password});
    final user = User.fromJson(
        (body as Map<String, dynamic>)['user'] as Map<String, dynamic>? ?? {});
    loginName = name;
    apiKey = user.apiKey;
    return user;
  }

  /// Verifies an API key by fetching the current user.
  Future<User> loginWithKey(String name, String key) async {
    loginName = name;
    apiKey = key;
    try {
      final body = await _get('/users/$name.json');
      return User.fromJson(body as Map<String, dynamic>);
    } catch (_) {
      loginName = null;
      apiKey = null;
      rethrow;
    }
  }

  Future<void> logout() => _send('DELETE', '/session.json');

  Future<AdminStats> adminStats() => _get('/admin/api/stats.json')
      .then((b) => AdminStats.fromJson(b as Map<String, dynamic>));

  Future<List<PostFlag>> flags() => _get('/post_flags.json',
          {'search[is_resolved]': 'false'}).then((body) => (body as List)
      .whereType<Map<String, dynamic>>()
      .map(PostFlag.fromJson)
      .toList());

  Future<void> resolveFlag(int id) =>
      _send('POST', '/admin/api/flags/$id/resolve.json');

  Future<List<User>> users() => _get('/users.json').then((body) => (body as List)
      .whereType<Map<String, dynamic>>()
      .map(User.fromJson)
      .toList());

  Future<void> createUser(String name, String password, int level) =>
      _send('POST', '/users.json',
          form: {'name': name, 'password': password, 'level': '$level'});

  Future<Map<String, dynamic>> health() => _get('/health.json')
      .then((b) => b as Map<String, dynamic>);
}
