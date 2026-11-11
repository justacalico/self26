import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'l10n/app_localizations.dart';
import 'state.dart';
import 'window.dart';

/// macOS-style traffic-light buttons: close / minimize / maximize on the
/// left, glyphs appear on hover.
class TrafficLights extends StatelessWidget {
  const TrafficLights({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        _Light(
          key: const Key('window_close'),
          color: const Color(0xFFFF5F57),
          icon: Icons.close,
          tooltip: l.windowClose,
          onPressed: windowActions.close,
        ),
        const SizedBox(width: 8),
        _Light(
          key: const Key('window_minimize'),
          color: const Color(0xFFFFBD2E),
          icon: Icons.remove,
          tooltip: l.windowMinimize,
          onPressed: windowActions.minimize,
        ),
        const SizedBox(width: 8),
        _Light(
          key: const Key('window_maximize'),
          color: const Color(0xFF28CA41),
          icon: Icons.add,
          tooltip: l.windowZoom,
          onPressed: windowActions.toggleMaximize,
        ),
      ],
    );
  }
}

class _Light extends StatefulWidget {
  final Color color;
  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  const _Light({
    super.key,
    required this.color,
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  @override
  State<_Light> createState() => _LightState();
}

class _LightState extends State<_Light> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        onTap: widget.onPressed,
        child: Tooltip(
          message: widget.tooltip,
          child: SizedBox(
            width: 24,
            height: 24,
            child: Center(
              child: Container(
                width: 13,
                height: 13,
                decoration:
                    BoxDecoration(color: widget.color, shape: BoxShape.circle),
                child: _hovered
                    ? Icon(widget.icon,
                        size: 9, color: Colors.black.withAlpha(140))
                    : null,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// The custom top bar rendered on desktop: traffic lights + window title +
/// a drag region that double-clicks to maximize/restore.
class DesktopTopBar extends StatelessWidget {
  final String title;
  const DesktopTopBar({super.key, required this.title});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return GestureDetector(
      onPanStart: (_) => windowActions.startDragging(),
      onDoubleTap: () => windowActions.toggleMaximize(),
      child: Container(
        height: 40,
        color: scheme.surfaceContainerLow,
        padding: const EdgeInsets.symmetric(horizontal: 12),
        child: Row(
          children: [
            const TrafficLights(),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                title,
                style: Theme.of(context).textTheme.bodyMedium,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// One sidebar destination: icon + label side by side in a pill that
/// highlights when selected.
class NavRow extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  const NavRow({
    super.key,
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
      child: Material(
        color: Colors.transparent,
        shape: const StadiumBorder(),
        clipBehavior: Clip.antiAlias,
        child: ListTile(
          dense: true,
          leading: Icon(icon, size: 20),
          title: Text(label),
          selected: selected,
          selectedTileColor: scheme.secondaryContainer,
          hoverColor: scheme.surfaceContainerHighest.withAlpha(120),
          shape: const StadiumBorder(),
          onTap: onTap,
        ),
      ),
    );
  }
}

class NavSidePanel extends StatelessWidget {
  const NavSidePanel({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();
    final scheme = Theme.of(context).colorScheme;

    final rows = <(NavDest, IconData, String)>[
      (NavDest.posts, Icons.grid_view, l.navPosts),
      (NavDest.favorites, Icons.favorite_border, l.navFavourites),
      (NavDest.pools, Icons.collections_bookmark_outlined, l.navPools),
      (NavDest.tags, Icons.sell_outlined, l.navTags),
      (NavDest.comments, Icons.chat_bubble_outline, l.navComments),
      (NavDest.upload, Icons.upload_outlined, l.navUpload),
      if (state.user?.isAdmin ?? false)
        (NavDest.admin, Icons.admin_panel_settings_outlined, l.navAdmin),
      (NavDest.settings, Icons.settings_outlined, l.navSettings),
    ];

    return Container(
      width: 240,
      color: scheme.surfaceContainerLow,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SizedBox(height: 8),
          Expanded(
            child: ListView(
              padding: EdgeInsets.zero,
              children: [
                for (final (dest, icon, label) in rows)
                  NavRow(
                    key: Key('nav_${dest.name}'),
                    icon: icon,
                    label: label,
                    selected: state.nav == dest,
                    onTap: () {
                      state.select(dest);
                      Scaffold.maybeOf(context)?.closeDrawer();
                    },
                  ),
              ],
            ),
          ),
          const Divider(height: 1),
          _AccountRow(),
        ],
      ),
    );
  }
}

class _AccountRow extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final state = context.watch<AppState>();
    final user = state.user;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      child: Material(
        color: Colors.transparent,
        shape: const StadiumBorder(),
        clipBehavior: Clip.antiAlias,
        child: ListTile(
          dense: true,
          leading: Icon(
            user == null ? Icons.login : Icons.account_circle_outlined,
            size: 20,
          ),
          title: Text(
            user == null ? l.navLogin : l.loggedInAs(user.name),
            overflow: TextOverflow.ellipsis,
          ),
          onTap: () {
            if (user == null) {
              state.select(NavDest.settings);
              state.setSearch('');
              Navigator.of(context).pushNamed('/login');
            } else {
              state.signOut();
            }
          },
        ),
      ),
    );
  }
}

/// The shared shell: custom desktop top bar, wide sidebar (drawer on
/// narrow layouts), and the active page body.
class AppShell extends StatelessWidget {
  final String title;
  final Widget body;

  const AppShell({super.key, required this.title, required this.body});

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width >= 840;
    return Scaffold(
      drawer: wide
          ? null
          : const Drawer(child: SafeArea(child: NavSidePanel())),
      body: Column(
        children: [
          if (isDesktopPlatform) DesktopTopBar(title: title),
          if (!wide) _NarrowHeader(title: title),
          Expanded(
            child: Row(
              children: [
                if (wide) const NavSidePanel(),
                Expanded(child: body),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _NarrowHeader extends StatelessWidget {
  final String title;
  const _NarrowHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Container(
      height: 48,
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: Row(
        children: [
          Builder(
            builder: (ctx) => IconButton(
              icon: const Icon(Icons.menu),
              tooltip: l.sidebarCollapse,
              onPressed: () => Scaffold.of(ctx).openDrawer(),
            ),
          ),
          Expanded(
            child: Text(title,
                style: Theme.of(context).textTheme.titleMedium,
                overflow: TextOverflow.ellipsis),
          ),
        ],
      ),
    );
  }
}
