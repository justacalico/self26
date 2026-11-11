import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'models.dart';
import 'version_utils.dart';

/// Thrown when the GitLab releases API cannot be fetched or decoded.
class VersionCheckException implements Exception {
  final String message;
  const VersionCheckException(this.message);

  @override
  String toString() => 'VersionCheckException: $message';
}

/// Checks GitLab Releases for the newest semver tag and compares it with
/// the installed version.
class VersionChecker {
  static const _host = 'gitlab.com';
  static const _projectPath = 'HttpAnimations/self26';
  static const _perPage = 100;
  static const _maxPages = 10;
  static const releasesUrl = 'https://$_host/$_projectPath/-/releases';

  http.Client? _client;
  final bool _ownsClient;

  VersionChecker({http.Client? client})
      : _client = client,
        _ownsClient = client == null;

  Future<AppUpdate> check(String currentVersion) async {
    final latest = await _fetchLatestRelease();
    if (latest == null) {
      return AppUpdate(
          currentVersion: currentVersion, releaseUrl: releasesUrl);
    }

    final current = Version.parse(currentVersion);
    final releaseUrl =
        '$releasesUrl/${Uri.encodeComponent(latest.tag)}';
    if (current == null) {
      return AppUpdate(
        currentVersion: currentVersion,
        latestVersion: latest.version.toString(),
        updateAvailable: true,
        releaseUrl: releaseUrl,
      );
    }
    return AppUpdate(
      currentVersion: currentVersion,
      latestVersion: latest.version.toString(),
      updateAvailable: latest.version.compareTo(current) > 0,
      releaseUrl: releaseUrl,
    );
  }

  void close() {
    if (_ownsClient) {
      _client?.close();
      _client = null;
    }
  }

  Future<({Version version, String tag})?> _fetchLatestRelease() async {
    Version? maxVersion;
    String? maxTag;
    for (var page = 1; page <= _maxPages; page++) {
      final uri = Uri(
        scheme: 'https',
        host: _host,
        pathSegments: ['api', 'v4', 'projects', _projectPath, 'releases'],
        queryParameters: {
          'per_page': '$_perPage',
          'order_by': 'released_at',
          'sort': 'desc',
          'page': '$page',
        },
      );

      try {
        final client = _client ??= http.Client();
        final response = await client
            .get(uri, headers: {'Accept': 'application/json'})
            .timeout(const Duration(seconds: 10));

        if (response.statusCode != 200) {
          throw VersionCheckException('HTTP ${response.statusCode}');
        }
        final decoded = jsonDecode(response.body);
        if (decoded is! List<dynamic>) {
          throw const VersionCheckException('unexpected response shape');
        }
        for (final release in decoded) {
          if (release is! Map<String, dynamic>) continue;
          final tag = release['tag_name'];
          if (tag is! String) continue;
          final version = Version.parse(tag);
          if (version == null) continue;
          if (maxVersion == null || version.compareTo(maxVersion) > 0) {
            maxVersion = version;
            maxTag = tag;
          }
        }
        if (decoded.length < _perPage) break;
      } catch (e) {
        if (e is VersionCheckException) rethrow;
        throw VersionCheckException('$e');
      }
    }
    if (maxVersion == null || maxTag == null) return null;
    return (version: maxVersion, tag: maxTag);
  }
}
