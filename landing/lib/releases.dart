import 'dart:convert';

import 'package:http/http.dart' as http;

const projectUrl = 'https://gitlab.com/HttpAnimations/self26';
const releasesPageUrl = '$projectUrl/-/releases';
const releasesApiUrl =
    'https://gitlab.com/api/v4/projects/HttpAnimations%2Fself26/releases?per_page=20';

class ReleaseAsset {
  const ReleaseAsset({required this.name, required this.url});

  final String name;
  final String url;
}

class LandingRelease {
  const LandingRelease(
      {required this.name, required this.tag, required this.assets});

  final String name;
  final String tag;
  final List<ReleaseAsset> assets;
}

Future<LandingRelease?> fetchLatestRelease(http.Client client) async {
  final res = await client.get(Uri.parse(releasesApiUrl));
  if (res.statusCode != 200) {
    throw Exception('releases request failed: ${res.statusCode}');
  }
  final body = jsonDecode(res.body);
  if (body is! List || body.isEmpty) return null;

  final first = body.first;
  if (first is! Map) return null;

  final assets = <ReleaseAsset>[];
  final links = first['assets'] is Map ? first['assets']['links'] : null;
  if (links is List) {
    for (final link in links) {
      if (link is Map && link['name'] is String && link['url'] is String) {
        assets.add(ReleaseAsset(name: link['name'], url: link['url']));
      }
    }
  }

  final name = first['name'] is String ? first['name'] as String : '';
  final tag = first['tag_name'] is String ? first['tag_name'] as String : '';
  return LandingRelease(name: name, tag: tag, assets: assets);
}
