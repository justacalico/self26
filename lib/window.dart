// coverage:ignore-file
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:window_manager/window_manager.dart';

/// Abstraction over `windowManager` so widgets are testable without a real
/// desktop window.
abstract class WindowActions {
  Future<void> close();
  Future<void> minimize();
  Future<void> toggleMaximize();
  Future<void> startDragging();
}

class WindowManagerActions implements WindowActions {
  const WindowManagerActions();

  @override
  Future<void> close() => windowManager.close();

  @override
  Future<void> minimize() => windowManager.minimize();

  @override
  Future<void> toggleMaximize() async {
    if (await windowManager.isMaximized()) {
      await windowManager.unmaximize();
    } else {
      await windowManager.maximize();
    }
  }

  @override
  Future<void> startDragging() => windowManager.startDragging();
}

/// The live implementation; tests replace this with a fake.
WindowActions windowActions = const WindowManagerActions();

/// Lets tests force the desktop code paths on or off regardless of host
/// platform.
bool? debugIsDesktop;

bool get isDesktopPlatform {
  final forced = debugIsDesktop;
  if (forced != null) return forced;
  if (kIsWeb) return false;
  return Platform.isLinux || Platform.isWindows || Platform.isMacOS;
}

/// Hides the native title bar on desktop builds so the app can draw its own
/// macOS-style chrome. Missing plugin implementations (tests, headless
/// shells) are ignored.
Future<void> initDesktopWindow() async {
  if (!isDesktopPlatform) return;
  try {
    await windowManager.ensureInitialized();
    const options = WindowOptions(
      titleBarStyle: TitleBarStyle.hidden,
      windowButtonVisibility: false,
    );
    await windowManager.waitUntilReadyToShow(options, () async {
      await windowManager.show();
      await windowManager.focus();
    });
  } on MissingPluginException {
    // No window_manager implementation (e.g. widget tests).
  }
}
