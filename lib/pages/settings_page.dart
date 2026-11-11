import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../l10n/app_localizations.dart';
import '../state.dart';
import '../version_checker.dart';

/// Settings: server, language, theme, and About with the update check.
class SettingsPage extends StatelessWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();

    return ListView(
      padding: const EdgeInsets.symmetric(vertical: 8),
      children: [
        _Section(l.settingsServer),
        ListTile(
          leading: const Icon(Icons.dns_outlined),
          title: Text(l.settingsServerUrl),
          subtitle: Text(state.api.baseUrl),
          onTap: () => _editServer(context, l, state),
        ),
        _Section(l.settingsLanguage),
        RadioGroup<Locale?>(
          groupValue: state.locale,
          onChanged: state.setLocale,
          child: Column(
            children: [
              for (final locale in appLocales)
                RadioListTile<Locale?>(
                  value: locale,
                  title: Text(locale == null
                      ? '${l.languageSystem} (${appLocaleNames[locale]})'
                      : appLocaleNames[locale]!),
                ),
            ],
          ),
        ),
        _Section(l.settingsTheme),
        RadioGroup<AppThemeMode>(
          groupValue: state.themeMode,
          onChanged: (m) => state.setThemeMode(m ?? AppThemeMode.system),
          child: Column(
            children: [
              for (final (mode, label) in [
                (AppThemeMode.system, l.themeSystem),
                (AppThemeMode.light, l.themeLight),
                (AppThemeMode.dark, l.themeDark),
                (AppThemeMode.oled, l.themeOled),
              ])
                RadioListTile<AppThemeMode>(
                  value: mode,
                  title: Text(label),
                ),
            ],
          ),
        ),
        _Section(l.settingsAbout),
        FutureBuilder<String>(
          future: state.appVersion(),
          builder: (context, snap) => ListTile(
            leading: const Icon(Icons.info_outline),
            title: Text(l.settingsVersion),
            subtitle: Text(snap.data ?? '…'),
          ),
        ),
        ListTile(
          leading: const Icon(Icons.system_update_alt),
          title: Text(l.settingsCheckUpdates),
          subtitle: _updateStatus(l, state),
          trailing: state.checkingUpdate
              ? const SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2))
              : null,
          onTap: state.checkingUpdate ? null : state.checkForUpdate,
        ),
        if (state.appUpdate?.updateAvailable ?? false)
          ListTile(
            leading: const Icon(Icons.open_in_new),
            title: Text(l.settingsViewRelease),
            onTap: () => state.openUrl(Uri.parse(state.appUpdate!.releaseUrl)),
          ),
        ListTile(
          leading: const Icon(Icons.history),
          title: Text(l.settingsReleases),
          onTap: () =>
              state.openUrl(Uri.parse(VersionChecker.releasesUrl)),
        ),
        ListTile(
          leading: const Icon(Icons.code),
          title: Text(l.settingsSource),
          onTap: () => state.openUrl(
              Uri.parse('https://gitlab.com/HttpAnimations/self26')),
        ),
        ListTile(
          leading: const Icon(Icons.gavel_outlined),
          title: Text(l.settingsLicence),
          onTap: () => state.openUrl(Uri.parse(
              'https://gitlab.com/HttpAnimations/self26/-/blob/main/LICENSE')),
        ),
      ],
    );
  }

  Widget? _updateStatus(AppLocalizations l, AppState state) {
    if (state.checkingUpdate) return Text(l.settingsChecking);
    if (state.updateCheckFailed) return Text(l.settingsCheckFailed);
    final update = state.appUpdate;
    if (update == null) return null;
    if (update.updateAvailable) {
      return Text(l.settingsUpdateAvailable(update.latestVersion));
    }
    return Text(l.settingsUpToDate);
  }

  void _editServer(BuildContext context, AppLocalizations l, AppState state) {
    final controller = TextEditingController(text: state.api.baseUrl);
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l.settingsServerUrl),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(hintText: 'https://example.com'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(ctx).pop();
              state.setServerUrl(controller.text);
            },
            child: Text(l.save),
          ),
        ],
      ),
    );
  }
}

class _Section extends StatelessWidget {
  final String title;
  const _Section(this.title);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
      child: Text(
        title,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
            color: Theme.of(context).colorScheme.primary),
      ),
    );
  }
}
