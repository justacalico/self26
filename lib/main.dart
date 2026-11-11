// coverage:ignore-file
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'app.dart';
import 'state.dart';
import 'window.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initDesktopWindow();
  final prefs = await SharedPreferences.getInstance();
  final state = AppState.defaults(prefs);
  runApp(ChangeNotifierProvider.value(value: state, child: const Self26App()));
  state.init();
}
