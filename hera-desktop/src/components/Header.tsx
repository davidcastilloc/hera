import { Popover } from '@base-ui/react/popover';
import * as stylex from '@stylexjs/stylex';
import { useRef } from 'react';

import ButtonIcon from '../elements/ButtonIcon';
import { usePlayerState } from '../hooks/usePlayer';
import usePlayingTrack from '../hooks/usePlayingTrack';
import PlayerControls from './PlayerControls';
import PlayingBar from './PlayingBar';
import Queue from './Queue';
import Search from './Search';

export default function Header() {
  const queue = usePlayerState((state) => state.queue);
  const queueCursor = usePlayerState((state) => state.queueCursor);
  const trackPlaying = usePlayingTrack();
  const platform =
    typeof window !== 'undefined' ? window.__MUSEEKS_PLATFORM : 'windows';
  const queueAnchorRef = useRef<HTMLDivElement>(null);

  return (
    <header
      aria-label="Transporte y búsqueda de cabina"
      {...stylex.props(styles.header)}
      data-tauri-drag-region
    >
      <div
        {...stylex.props(
          styles.headerMainControls,
          platform === 'macos' && styles.headerMainControlsMacos,
        )}
        data-tauri-drag-region
      >
        <PlayerControls />
      </div>

      <div {...stylex.props(styles.headerPlayingBar)} data-tauri-drag-region>
        {trackPlaying != null ? (
          <>
            <PlayingBar trackPlaying={trackPlaying} />
            <Popover.Root>
              <div ref={queueAnchorRef} {...stylex.props(styles.queue)}>
                <Popover.Trigger
                  render={(triggerProps) => (
                    <ButtonIcon
                      {...triggerProps}
                      icon="list"
                      iconSize={18}
                      label="Cola de reproducción"
                      data-tauri-drag-region
                    />
                  )}
                />
              </div>
              <Popover.Portal>
                <Popover.Positioner
                  side="bottom"
                  alignOffset={-16}
                  align="end"
                  anchor={queueAnchorRef}
                >
                  <Popover.Popup {...stylex.props(styles.queueContainer)}>
                    <Queue queue={queue} queueCursor={queueCursor} />
                  </Popover.Popup>
                </Popover.Positioner>
              </Popover.Portal>
            </Popover.Root>
          </>
        ) : (
          <div {...stylex.props(styles.idleTransport)} data-tauri-drag-region>
            Preescucha lista · Doble clic en cualquier pista para activar 4 Hot
            Cues
          </div>
        )}
      </div>

      <div {...stylex.props(styles.headerSearch)} data-tauri-drag-region>
        <Search />
      </div>
    </header>
  );
}

const styles = stylex.create({
  header: {
    boxSizing: 'border-box',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    backgroundColor: '#12161d',
    color: '#edeff3',
    paddingBlock: 0,
    paddingInline: '14px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '16px',
    height: '52px',
    flexGrow: 0,
    flexShrink: 0,
    flexBasis: '52px',
  },
  headerMainControls: {
    flexGrow: 0,
    flexShrink: 0,
    flexBasis: 'auto',
    display: 'flex',
    alignItems: 'center',
    minWidth: '196px',
    paddingLeft: '4px',
  },
  headerMainControlsMacos: {
    paddingLeft: '72px',
  },
  headerPlayingBar: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: 'auto',
    minWidth: 0,
    maxWidth: '640px',
    height: '100%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  idleTransport: {
    fontSize: '12px',
    color: '#8e98a8',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  headerSearch: {
    flexGrow: 0,
    flexShrink: 0,
    flexBasis: 'auto',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'flex-end',
  },
  queue: {
    marginBlock: 0,
    marginInline: '12px',
    display: 'flex',
    alignItems: 'center',
  },
  queueContainer: {
    zIndex: 1000,
    display: {
      ':is([data-open])': 'block',
    },
  },
});
