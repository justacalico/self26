import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'l10n/app_localizations.dart';
import 'pages/admin_page.dart';
import 'pages/comments_page.dart';
import 'pages/favorites_page.dart';
import 'pages/login_page.dart';
import 'pages/pools_page.dart';
import 'pages/post_page.dart';
import 'pages/posts_page.dart';
import 'pages/settings_page.dart';
import 'pages/tags_page.dart';
import 'pages/upload_page.dart';
import 'shell.dart';
import 'state.dart';

const seedColor = Color(0xFF00549F);

ThemeData buildTheme(Brightness brightness, {bool oled = false}) {
  final scheme = ColorScheme.fromSeed(
    seedColor: seedColor,
    brightness: brightness,
  );
  final effective = oled && brightness == Brightness.dark
      ? scheme.copyWith(
          surface: Colors.black,
          surfaceContainerLowest: Colors.black,
          surfaceContainerLow: const Color(0xFF0A0A0A),
          surfaceContainer: const Color(0xFF101010),
          surfaceContainerHigh: const Color(0xFF161616),
          surfaceContainerHighest: const Color(0xFF1E1E1E),
        )
      : scheme;
  return ThemeData(
    colorScheme: effective,
    fontFamily: 'NotoSans',
    useMaterial3: true,
  );
}

class Self26App extends StatelessWidget {
  const Self26App({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<AppState>();
    return MaterialApp(
      title: 'self26',
      theme: buildTheme(Brightness.light),
      darkTheme: buildTheme(Brightness.dark, oled: state.themeMode == AppThemeMode.oled),
      themeMode: state.materialThemeMode,
      locale: state.locale,
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      routes: {'/login': (_) => const LoginPage()},
      home: const _Home(),
    );
  }
}

class _Home extends StatelessWidget {
  const _Home();

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();
    final post = state.selectedPost;
    if (post != null) return PostPage(post: post);

    final (title, body) = switch (state.nav) {
      NavDest.posts => (l.navPosts, const PostsPage()),
      NavDest.favorites => (l.navFavourites, const FavoritesPage()),
      NavDest.pools => (l.navPools, const PoolsPage()),
      NavDest.tags => (l.navTags, const TagsPage()),
      NavDest.comments => (l.navComments, const CommentsPage()),
      NavDest.upload => (l.navUpload, const UploadPage()),
      NavDest.admin => (l.navAdmin, const AdminPage()),
      NavDest.settings => (l.navSettings, const SettingsPage()),
    };
    return AppShell(title: 'self26 · $title', body: body);
  }
}
