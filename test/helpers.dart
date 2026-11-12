import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:provider/provider.dart';
import 'package:self26/app.dart';
import 'package:self26/api.dart';
import 'package:self26/models.dart';
import 'package:self26/state.dart';
import 'package:self26/version_checker.dart';
import 'package:self26/window.dart';
import 'package:shared_preferences/shared_preferences.dart';

typedef RouteHandler = Object? Function(http.BaseRequest request);

class FakeClient extends http.BaseClient {
  FakeClient([Map<String, Object?> routes = const {}])
      : routes = {..._defaultRoutes, ...routes};

  final Map<String, Object?> routes;
  final requests = <http.BaseRequest>[];

  static final _defaultRoutes = <String, Object?>{
    'GET /health.json': {'ok': true},
    'GET /posts.json': {
      'posts': [postJson(1), 42, barePostJson(2)],
    },
    'GET /explore/posts/popular.json': {
      'posts': [postJson(1)],
    },
    'GET /posts/0.json': barePostJson(0),
    'GET /posts/1.json': postJson(1),
    'GET /posts/2.json': barePostJson(2),
    'GET /favorites.json': {
      'posts': [postJson(1)],
    },
    'GET /pools.json': [poolJson(2), barePoolJson(3)],
    'GET /pools/2/posts.json': {
      'posts': [postJson(1)],
    },
    'GET /pools/3/posts.json': {'posts': <Object?>[]},
    'GET /tags.json': [
      tagJson('fox', 1),
      tagJson('movie', 3),
      tagJson('wolf', 5),
      {'value': 'anon'},
    ],
    'GET /tags/autocomplete.json': [tagJson('fox', 1)],
    'GET /comments.json': [commentJson()],
    'POST /posts/1/votes.json': <String, Object?>{},
    'POST /posts/2/votes.json': <String, Object?>{},
    'POST /favorites.json': <String, Object?>{},
    'DELETE /favorites/1.json': '',
    'DELETE /posts/1.json': '',
    'DELETE /posts/2.json': '',
    'POST /comments.json': <String, Object?>{},
    'DELETE /comments/1.json': '',
    'POST /uploads.json': {'post_id': 1},
    'POST /session.json': {'user': userJson()},
    'DELETE /session.json': '',
    'GET /users/alice.json': userJson(),
    'GET /users.json': [userJson(), userJson(admin: true)],
    'POST /users.json': <String, Object?>{},
    'GET /admin/api/stats.json': statsJson(),
    'GET /post_flags.json': [flagJson()],
    'POST /admin/api/flags/1/resolve.json': <String, Object?>{},
    'POST /pools.json': poolJson(2),
  };

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    requests.add(request);
    Object? handler = routes['${request.method} ${request.url.path}'];
    while (handler is RouteHandler) {
      handler = handler(request);
    }
    if (handler is Future<Object?>) handler = await handler;
    if (handler is Exception) throw handler;
    if (handler == null) {
      return _respond(request, 404, '{"success":false}');
    }
    if (handler is int) return _respond(request, handler, 'error $handler');
    final body = handler is String ? handler : jsonEncode(handler);
    return _respond(request, 200, body);
  }

  http.StreamedResponse _respond(
      http.BaseRequest request, int status, String body) {
    return http.StreamedResponse(
      Stream<List<int>>.value(utf8.encode(body)),
      status,
      request: request,
      headers: const {'content-type': 'application/json'},
    );
  }
}

class FakeWindowActions implements WindowActions {
  final calls = <String>[];

  @override
  Future<void> close() async => calls.add('close');

  @override
  Future<void> minimize() async => calls.add('minimize');

  @override
  Future<void> toggleMaximize() async => calls.add('toggleMaximize');

  @override
  Future<void> startDragging() async => calls.add('startDragging');
}

FakeWindowActions installFakeWindowActions() {
  final actions = FakeWindowActions();
  windowActions = actions;
  addTearDown(() => windowActions = const WindowManagerActions());
  return actions;
}

void forceDesktop(bool? value) {
  debugIsDesktop = value;
  addTearDown(() => debugIsDesktop = null);
}

Map<String, Object?> postJson(
  int id, {
  bool video = false,
  bool favorited = false,
  int vote = 0,
}) =>
    {
      'id': id,
      'created_at': '2024-01-01T00:00:00.000Z',
      'rating': 's',
      'uploader_id': 7,
      'uploader_name': 'alice',
      'description': 'a description',
      'sources': ['https://example.com/$id'],
      'pools': [2],
      'files': {
        'meta': {'md5': 'abc$id', 'ext': video ? 'webm' : 'png', 'size': 4096},
        'original': {
          'width': 800,
          'height': 600,
          'url': 'http://img.test/$id.png'
        },
        'preview': {
          'jpg': 'http://img.test/p$id.jpg',
          'width': 150,
          'height': 150
        },
        'sample': {'jpg': 'http://img.test/s$id.jpg'},
      },
      'stats': {
        'score': {'up': 5, 'down': 2},
        'fav_count': 3,
        'is_favorited': favorited,
        'vote': vote,
        'comment_count': 2,
      },
      'has': {'sample': true},
      'relationships': {
        'parent_id': 9,
        'children': [4, 5],
      },
      'tags': {
        'artist': ['bob'],
        'general': ['cat_ears', 'solo'],
      },
      'flags': {'pending': true, 'flagged': true},
    };

Map<String, Object?> barePostJson(int id) => {
      'id': id,
      'tags': ['one_tag', 'two_tag'],
    };

Map<String, Object?> commentJson({int creatorId = 7}) => {
      'id': 1,
      'post_id': 1,
      'creator_id': creatorId,
      'creator_name': 'alice',
      'body': 'nice post',
      'score': 2,
      'created_at': '2024-01-01T00:00:00.000Z',
    };

Map<String, Object?> poolJson(int id) => {
      'id': id,
      'name': 'cool_pool',
      'description': 'a series',
      'category': 'series',
      'creator_id': 7,
      'creator_name': 'alice',
      'post_ids': [1, 2, 3],
      'is_active': true,
    };

Map<String, Object?> barePoolJson(int id) =>
    {'id': id, 'name': 'empty_pool', 'is_active': false};

Map<String, Object?> tagJson(String name, int category) => {
      'id': 1,
      'name': name,
      'category': category,
      'post_count': 42,
    };

Map<String, Object?> userJson({bool admin = false}) => {
      'id': admin ? 9 : 7,
      'name': admin ? 'root' : 'alice',
      'level': admin ? 55 : 20,
      'api_key': 'key123',
    };

Map<String, Object?> flagJson() => {
      'id': 1,
      'post_id': 1,
      'reason': 'spam',
      'is_resolved': false,
    };

Map<String, Object?> statsJson() => {
      'posts': 10,
      'users': 2,
      'tags': 5,
      'comments': 3,
      'pools': 1,
      'pending_flags': 2,
      'pending_tickets': 1,
      'disk_bytes': 2000000000,
    };

const adminUser = User(id: 9, name: 'root', level: 55);
const memberUser = User(id: 7, name: 'alice', level: 20);

Future<AppState> makeState({
  Map<String, Object> prefs = const {},
  FakeClient? client,
  String baseUrl = 'http://server.test',
  User? user,
  VersionChecker? versionChecker,
  String appVersion = '1.0.0',
  void Function(Uri url)? onOpen,
  bool injectDefaults = true,
}) async {
  SharedPreferences.setMockInitialValues(prefs);
  final p = await SharedPreferences.getInstance();
  final c = client ?? FakeClient();
  if (!injectDefaults) {
    return AppState(api: Self26Api(baseUrl, client: c), prefs: p, user: user);
  }
  return AppState(
    api: Self26Api(baseUrl, client: c),
    prefs: p,
    versionChecker: versionChecker ?? VersionChecker(client: c),
    appVersion: () async => appVersion,
    openUrl: (url) async => onOpen?.call(url),
    user: user,
  );
}

Future<AppState> pumpApp(
  WidgetTester tester, {
  Map<String, Object> prefs = const {},
  FakeClient? client,
  String baseUrl = 'http://server.test',
  User? user,
  VersionChecker? versionChecker,
  String appVersion = '1.0.0',
  void Function(Uri url)? onOpen,
  Size size = const Size(1280, 800),
  bool? desktop,
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
  if (desktop != null) forceDesktop(desktop);
  final state = await makeState(
    prefs: prefs,
    client: client,
    baseUrl: baseUrl,
    user: user,
    versionChecker: versionChecker,
    appVersion: appVersion,
    onOpen: onOpen,
  );
  await tester.pumpWidget(
    ChangeNotifierProvider.value(value: state, child: const Self26App()),
  );
  return state;
}

Future<void> loadNotoSans() async {
  final loader = FontLoader('NotoSans')
    ..addFont(rootBundle.load('assets/fonts/NotoSans-Regular.ttf'))
    ..addFont(rootBundle.load('assets/fonts/NotoSans-Medium.ttf'))
    ..addFont(rootBundle.load('assets/fonts/NotoSans-Bold.ttf'));
  await loader.load();
}
