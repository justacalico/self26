import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;

import 'l10n/app_localizations.dart';
import 'landing_page.dart';
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

class LandingApp extends StatelessWidget {
  const LandingApp({super.key, required this.state, this.client});

  final LandingState state;
  final http.Client? client;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: state,
      builder: (context, _) {
        return MaterialApp(
          title: 'self26',
          theme: buildTheme(Brightness.light),
          darkTheme: buildTheme(Brightness.dark,
              oled: state.themeMode == LandingThemeMode.oled),
          themeMode: state.materialThemeMode,
          locale: state.locale,
          supportedLocales: AppLocalizations.supportedLocales,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          home: LandingPage(state: state, client: client),
        );
      },
    );
  }
}
