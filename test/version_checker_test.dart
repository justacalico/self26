import 'package:flutter_test/flutter_test.dart';
import 'package:self26/version_checker.dart';

import 'helpers.dart';

const releasesRoute = 'GET /api/v4/projects/HttpAnimations%2Fself26/releases';

VersionChecker checker(Map<String, Object?> routes) =>
    VersionChecker(client: FakeClient(routes));

void main() {
  test('reports update available when latest is newer', () async {
    final update = await checker({
      releasesRoute: [
        {'tag_name': 'v2.0.0'},
        {'tag_name': 'v1.5.0'},
      ],
    }).check('1.0.0');
    expect(update.updateAvailable, isTrue);
    expect(update.currentVersion, '1.0.0');
    expect(update.latestVersion, '2.0.0');
    expect(update.releaseUrl,
        '${VersionChecker.releasesUrl}/v2.0.0');
  });

  test('reports no update when current is same or newer', () async {
    final same = await checker({
      releasesRoute: [
        {'tag_name': 'v1.0.0'},
      ],
    }).check('1.0.0');
    expect(same.updateAvailable, isFalse);
    expect(same.latestVersion, '1.0.0');

    final newer = await checker({
      releasesRoute: [
        {'tag_name': 'v1.0.0'},
      ],
    }).check('9.9.9');
    expect(newer.updateAvailable, isFalse);
  });

  test('unparseable current version still reports latest', () async {
    final update = await checker({
      releasesRoute: [
        {'tag_name': 'v2.0.0'},
      ],
    }).check('not-a-version');
    expect(update.updateAvailable, isTrue);
    expect(update.latestVersion, '2.0.0');
    expect(update.currentVersion, 'not-a-version');
  });

  test('returns empty result when no release tag parses', () async {
    final update = await checker({
      releasesRoute: [
        'a string',
        42,
        {'tag_name': 'not-semver'},
        {'tag_name': 7},
        {'name': 'no tag'},
      ],
    }).check('1.0.0');
    expect(update.updateAvailable, isFalse);
    expect(update.latestVersion, '');
    expect(update.releaseUrl, VersionChecker.releasesUrl);
  });

  test('paginates until a short page', () async {
    final pages = <String>[];
    final update = await checker({
      releasesRoute: (request) {
        final page = request.url.queryParameters['page'] ?? '?';
        pages.add(page);
        if (page == '1') {
          return List.generate(
              100, (i) => {'tag_name': 'v1.0.$i'});
        }
        return [
          {'tag_name': 'v3.0.0'},
        ];
      },
    }).check('1.0.0');
    expect(pages, ['1', '2']);
    expect(update.latestVersion, '3.0.0');
    expect(update.updateAvailable, isTrue);
  });

  test('throws on non-200 responses', () async {
    final c = checker({releasesRoute: 500});
    expect(() => c.check('1.0.0'),
        throwsA(isA<VersionCheckException>()));
    try {
      await c.check('1.0.0');
      fail('should have thrown');
    } on VersionCheckException catch (e) {
      expect(e.toString(), contains('HTTP 500'));
      expect(e.message, 'HTTP 500');
    }
  });

  test('throws on unexpected response shape', () async {
    final c = checker({releasesRoute: {'releases': []}});
    try {
      await c.check('1.0.0');
      fail('should have thrown');
    } on VersionCheckException catch (e) {
      expect(e.message, 'unexpected response shape');
    }
  });

  test('wraps transport errors', () async {
    final c = checker({releasesRoute: Exception('boom')});
    try {
      await c.check('1.0.0');
      fail('should have thrown');
    } on VersionCheckException catch (e) {
      expect(e.message, contains('boom'));
    }
  });

  test('close releases an owned client', () {
    final c = VersionChecker();
    c.close();
    c.close();
  });
}
