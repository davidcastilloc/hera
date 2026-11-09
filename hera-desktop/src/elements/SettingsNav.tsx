import { NavigationMenu } from '@base-ui/react/navigation-menu';
import * as stylex from '@stylexjs/stylex';
import { Link } from '@tanstack/react-router';
import type React from 'react';

interface WrapProps {
  children: React.ReactNode;
}

/**
 * Horizontal studio sub-navigation for Settings
 */
export function SettingsNav(props: WrapProps) {
  return (
    <NavigationMenu.Root orientation="horizontal">
      <NavigationMenu.List {...stylex.props(styles.nav)}>
        {props.children}
      </NavigationMenu.List>
    </NavigationMenu.Root>
  );
}

interface LinkProps {
  children: React.ReactNode;
  to: string;
}

/**
 * Single settings sub-navigation tab
 */
export function SettingsNavLink(props: LinkProps) {
  return (
    <NavigationMenu.Item>
      <NavigationMenu.Link
        render={(renderProps) => (
          <Link
            to={props.to}
            draggable={false}
            {...renderProps}
            {...stylex.props(styles.link)}
          >
            {props.children}
          </Link>
        )}
      />
    </NavigationMenu.Item>
  );
}

const styles = stylex.create({
  nav: {
    display: 'flex',
    flexDirection: 'row',
    alignItems: 'center',
    columnGap: '6px',
    listStyle: 'none',
    margin: 0,
    padding: 0,
  },
  link: {
    display: 'inline-flex',
    alignItems: 'center',
    paddingBlock: '6px',
    paddingInline: '12px',
    borderRadius: '6px',
    fontSize: '12.5px',
    fontWeight: {
      default: 500,
      ':is([data-status="active"])': 600,
    },
    textDecoration: 'none',
    whiteSpace: 'nowrap',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: 'transparent',
      ':is([data-status="active"])': 'rgba(20, 184, 166, 0.32)',
    },
    backgroundColor: {
      default: 'transparent',
      ':hover': 'rgba(237, 239, 243, 0.04)',
      ':is([data-status="active"])': 'rgba(20, 184, 166, 0.12)',
    },
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
      ':is([data-status="active"])': '#2dd4bf',
    },
    transition: 'background-color 0.14s ease, color 0.14s ease',
  },
});
