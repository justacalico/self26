import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:url_launcher/url_launcher.dart' as launcher;

import 'l10n/app_localizations.dart';
import 'releases.dart';
import 'showcase.dart';
import 'state.dart';

class LandingPage extends StatelessWidget {
  const LandingPage({super.key, required this.state, this.client});

  final LandingState state;
  final http.Client? client;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      body: Column(
        children: [
          _TopBar(state: state),
          Container(height: 1, color: scheme.outlineVariant),
          Expanded(
            child: SingleChildScrollView(
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 980),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 24),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const SizedBox(height: 72),
                        const _Hero(),
                        const SizedBox(height: 72),
                        _SectionTitle(l.showcaseTitle),
                        const SizedBox(height: 24),
                        const _Showcase(),
                        const SizedBox(height: 72),
                        _SectionTitle(l.featuresTitle),
                        const SizedBox(height: 16),
                        const _Features(),
                        const SizedBox(height: 72),
                        _SectionTitle(l.downloadsTitle),
                        const SizedBox(height: 16),
                        _Downloads(client: client),
                        const SizedBox(height: 72),
                        const _Footer(),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

Uri _appUri() => Uri.base.resolve('app/');
Uri _flatpakUri() => Uri.base.resolve('self26.flatpakref');

Future<void> _open(Uri uri, {bool newTab = false}) {
  return launcher.launchUrl(uri,
      webOnlyWindowName: newTab ? '_blank' : null);
}

class _TopBar extends StatelessWidget {
  const _TopBar({required this.state});

  final LandingState state;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    return SizedBox(
      height: 60,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          children: [
            Icon(Icons.pets, color: scheme.primary, size: 22),
            const SizedBox(width: 8),
            Text(
              'self26',
              style: Theme.of(context)
                  .textTheme
                  .titleLarge
                  ?.copyWith(fontWeight: FontWeight.w700),
            ),
            const Spacer(),
            _LanguageMenu(state: state),
            _ThemeMenu(state: state),
            const SizedBox(width: 8),
            FilledButton.icon(
              onPressed: () => _open(_appUri()),
              icon: const Icon(Icons.open_in_new, size: 16),
              label: Text(l.openApp),
            ),
          ],
        ),
      ),
    );
  }
}

class _ThemeMenu extends StatelessWidget {
  const _ThemeMenu({required this.state});

  final LandingState state;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final labels = {
      LandingThemeMode.system: l.themeSystem,
      LandingThemeMode.light: l.themeLight,
      LandingThemeMode.dark: l.themeDark,
      LandingThemeMode.oled: l.themeOled,
    };
    return PopupMenuButton<LandingThemeMode>(
      tooltip: l.themeLabel,
      icon: const Icon(Icons.brightness_6_outlined),
      onSelected: state.setThemeMode,
      itemBuilder: (context) => [
        for (final mode in LandingThemeMode.values)
          CheckedPopupMenuItem(
            value: mode,
            checked: state.themeMode == mode,
            child: Text(labels[mode]!),
          ),
      ],
    );
  }
}

class _LanguageMenu extends StatelessWidget {
  const _LanguageMenu({required this.state});

  final LandingState state;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return PopupMenuButton<Locale?>(
      tooltip: l.languageLabel,
      icon: const Icon(Icons.translate),
      onSelected: state.setLocale,
      itemBuilder: (context) => [
        for (final locale in landingLocales)
          CheckedPopupMenuItem(
            value: locale,
            checked: state.locale == locale,
            child: Text(locale == null
                ? l.langSystem
                : landingLocaleNames[locale]!),
          ),
      ],
    );
  }
}

class _Hero extends StatelessWidget {
  const _Hero();

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Column(
      children: [
        Text(
          'self26',
          textAlign: TextAlign.center,
          style: theme.textTheme.displayMedium?.copyWith(
            fontWeight: FontWeight.w700,
            color: theme.colorScheme.primary,
          ),
        ),
        const SizedBox(height: 12),
        Text(
          l.pitch,
          textAlign: TextAlign.center,
          style: theme.textTheme.headlineSmall,
        ),
        const SizedBox(height: 8),
        Text(
          l.heroSub,
          textAlign: TextAlign.center,
          style: theme.textTheme.bodyLarge
              ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
        ),
        const SizedBox(height: 28),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          alignment: WrapAlignment.center,
          children: [
            FilledButton.icon(
              onPressed: () => _open(_appUri()),
              icon: const Icon(Icons.open_in_new, size: 18),
              label: Text(l.openApp),
            ),
            OutlinedButton.icon(
              onPressed: () => _open(Uri.parse(projectUrl), newTab: true),
              icon: const Icon(Icons.code, size: 18),
              label: Text(l.viewOnGitLab),
            ),
          ],
        ),
      ],
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.title);

  final String title;

  @override
  Widget build(BuildContext context) {
    return Text(
      title,
      style: Theme.of(context)
          .textTheme
          .headlineMedium
          ?.copyWith(fontWeight: FontWeight.w600),
    );
  }
}

class _Showcase extends StatelessWidget {
  const _Showcase();

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    return Wrap(
      spacing: 24,
      runSpacing: 24,
      children: [
        ShowcaseCard(
          painter: PostGridPainter(scheme: scheme),
          caption: l.showcaseGridCaption,
        ),
        ShowcaseCard(
          painter: ApiCardPainter(scheme: scheme),
          caption: l.showcaseApiCaption,
        ),
        ShowcaseCard(
          painter: ShellPainter(scheme: scheme),
          caption: l.showcaseShellCaption,
        ),
      ],
    );
  }
}

class _Features extends StatelessWidget {
  const _Features();

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final rows = [
      (Icons.api, l.featureApi),
      (Icons.dns_outlined, l.featureSelfHost),
      (Icons.devices_outlined, l.featurePlatforms),
      (Icons.local_offer_outlined, l.featureBooru),
    ];
    return Column(
      children: [
        for (final (icon, text) in rows)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Row(
              children: [
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(
                    color: scheme.primaryContainer,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Icon(icon, size: 18, color: scheme.onPrimaryContainer),
                ),
                const SizedBox(width: 14),
                Expanded(
                  child:
                      Text(text, style: Theme.of(context).textTheme.bodyLarge),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _Downloads extends StatefulWidget {
  const _Downloads({this.client});

  final http.Client? client;

  @override
  State<_Downloads> createState() => _DownloadsState();
}

class _DownloadsState extends State<_Downloads> {
  late Future<({LandingRelease? release, bool failed})> _future;
  http.Client? _ownedClient;

  http.Client get _client => widget.client ?? (_ownedClient ??= http.Client());

  @override
  void initState() {
    super.initState();
    _future = _load();
  }

  Future<({LandingRelease? release, bool failed})> _load() async {
    try {
      return (release: await fetchLatestRelease(_client), failed: false);
    } catch (_) {
      return (release: null, failed: true);
    }
  }

  @override
  void dispose() {
    _ownedClient?.close();
    super.dispose();
  }

  void _retry() {
    setState(() {
      _future = _load();
    });
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        FutureBuilder<({LandingRelease? release, bool failed})>(
          future: _future,
          builder: (context, snap) {
            if (snap.connectionState != ConnectionState.done) {
              return const Padding(
                padding: EdgeInsets.symmetric(vertical: 16),
                child: Center(child: CircularProgressIndicator()),
              );
            }
            if (snap.hasError || snap.data!.failed) {
              return Row(
                children: [
                  Text(l.downloadsError,
                      style: theme.textTheme.bodyMedium
                          ?.copyWith(color: theme.colorScheme.error)),
                  const SizedBox(width: 12),
                  TextButton(onPressed: _retry, child: Text(l.retry)),
                ],
              );
            }
            final release = snap.data!.release;
            if (release == null) {
              return Text(l.downloadsEmpty,
                  style: theme.textTheme.bodyMedium
                      ?.copyWith(color: theme.colorScheme.onSurfaceVariant));
            }
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  release.name.isNotEmpty ? release.name : release.tag,
                  style: theme.textTheme.titleMedium
                      ?.copyWith(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 10),
                if (release.assets.isEmpty)
                  Text(l.downloadsEmpty,
                      style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant))
                else
                  Wrap(
                    spacing: 10,
                    runSpacing: 10,
                    children: [
                      for (final asset in release.assets)
                        OutlinedButton.icon(
                          onPressed: () =>
                              _open(Uri.parse(asset.url), newTab: true),
                          icon: const Icon(Icons.download, size: 16),
                          label: Text(asset.name),
                        ),
                    ],
                  ),
                const SizedBox(height: 16),
              ],
            );
          },
        ),
        Wrap(
          spacing: 10,
          runSpacing: 10,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            OutlinedButton.icon(
              onPressed: () =>
                  _open(Uri.parse(releasesPageUrl), newTab: true),
              icon: const Icon(Icons.new_releases_outlined, size: 16),
              label: Text(l.allReleases),
            ),
            Text(l.flatpakHint, style: theme.textTheme.bodyMedium),
            TextButton(
              onPressed: () => _open(_flatpakUri()),
              child: const Text('self26.flatpakref'),
            ),
          ],
        ),
      ],
    );
  }
}

class _Footer extends StatelessWidget {
  const _Footer();

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Column(
      children: [
        Container(height: 1, color: theme.colorScheme.outlineVariant),
        const SizedBox(height: 20),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          alignment: WrapAlignment.center,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            Text(
              l.footerLicence,
              style: theme.textTheme.bodySmall
                  ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
            ),
            TextButton(
              onPressed: () => _open(Uri.parse(projectUrl), newTab: true),
              child: Text(l.sourceCode),
            ),
          ],
        ),
        const SizedBox(height: 28),
      ],
    );
  }
}
