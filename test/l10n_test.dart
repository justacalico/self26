import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:self26/l10n/app_localizations.dart';

void main() {
  test('every supported locale exposes all strings', () {
    for (final locale in [
      const Locale('en'),
      const Locale('en', 'GB'),
      const Locale('zh'),
      const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
      const Locale('ru'),
      const Locale('ga'),
    ]) {
      final l = lookupAppLocalizations(locale);
      for (final value in allStrings(l)) {
        expect(value, isNotEmpty, reason: '${locale.toString()}: $value');
      }
    }
  });

  test('unsupported locale throws', () {
    expect(() => lookupAppLocalizations(const Locale('fr')),
        throwsA(isA<FlutterError>()));
  });

  test('delegate metadata', () async {
    const delegate = AppLocalizations.delegate;
    expect(delegate.isSupported(const Locale('en')), isTrue);
    expect(delegate.isSupported(const Locale('fr')), isFalse);
    expect((delegate as dynamic).shouldReload(delegate), isFalse);
    final loaded = await delegate.load(const Locale('ga'));
    expect(loaded.navPosts, isNotEmpty);
    expect(AppLocalizations.supportedLocales.length, 6);
    expect(AppLocalizations.localizationsDelegates, isNotEmpty);
  });

  testWidgets('AppLocalizations.of resolves through the app', (tester) async {
    late AppLocalizations l;
    await tester.pumpWidget(MaterialApp(
      locale: const Locale('ru'),
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      home: Builder(builder: (context) {
        l = AppLocalizations.of(context);
        return const SizedBox();
      }),
    ));
    expect(l.navPosts, 'Посты');
  });
}

List<String> allStrings(AppLocalizations l) => [
      l.localeName,
      l.appTitle,
      l.navPosts,
      l.navFavourites,
      l.navPools,
      l.navTags,
      l.navComments,
      l.navUpload,
      l.navAdmin,
      l.navSettings,
      l.navLogin,
      l.navLogout,
      l.searchHint,
      l.search,
      l.random,
      l.popular,
      l.hot,
      l.noPosts,
      l.noPostsHint,
      l.noFavourites,
      l.noPools,
      l.noComments,
      l.loading,
      l.loadFailed('thing'),
      l.retry,
      l.postScore,
      l.postFavourites,
      l.postComments,
      l.postUploader,
      l.postRating,
      l.postSources,
      l.postDescription,
      l.postTags,
      l.postPools,
      l.postParent,
      l.postChildren,
      l.addFavourite,
      l.removeFavourite,
      l.voteUp,
      l.voteDown,
      l.addComment,
      l.commentHint,
      l.commentPost,
      l.commentDelete,
      l.deletePost,
      l.deleteConfirm,
      l.cancel,
      l.confirm,
      l.save,
      l.editPost,
      l.uploadTitle,
      l.uploadChoose,
      l.uploadNoFile,
      l.uploadTags,
      l.uploadTagsHint,
      l.uploadSource,
      l.uploadDescription,
      l.uploadSubmit,
      l.uploadSuccess(7),
      l.uploadFailed,
      l.uploadSignIn,
      l.poolsNew,
      l.poolName,
      l.poolDescription,
      l.poolCreate,
      l.poolPosts(3),
      l.settingsTitle,
      l.settingsServer,
      l.settingsServerUrl,
      l.settingsLanguage,
      l.languageSystem,
      l.settingsTheme,
      l.themeSystem,
      l.themeLight,
      l.themeDark,
      l.themeOled,
      l.settingsAbout,
      l.settingsVersion,
      l.settingsCheckUpdates,
      l.settingsChecking,
      l.settingsUpToDate,
      l.settingsUpdateAvailable('9.9.9'),
      l.settingsCheckFailed,
      l.settingsViewRelease,
      l.settingsReleases,
      l.settingsLicence,
      l.settingsSource,
      l.loginTitle,
      l.loginName,
      l.loginPassword,
      l.loginSubmit,
      l.loginFailed,
      l.loginApiKey,
      l.loginApiKeyHint,
      l.loginUseKey,
      l.loggedInAs('alice'),
      l.adminTitle,
      l.adminStats,
      l.adminStatsPosts,
      l.adminStatsUsers,
      l.adminStatsTags,
      l.adminStatsComments,
      l.adminStatsPools,
      l.adminStatsFlags,
      l.adminStatsTickets,
      l.adminStatsDisk,
      l.adminUsers,
      l.adminFlags,
      l.adminResolve,
      l.adminNewUser,
      l.adminUserName,
      l.adminUserLevel,
      l.adminCreate,
      l.windowClose,
      l.windowMinimize,
      l.windowZoom,
      l.sidebarCollapse,
      l.emptyGeneric,
      l.errorOffline,
      l.ratingSafe,
      l.ratingQuestionable,
      l.ratingExplicit,
    ];
