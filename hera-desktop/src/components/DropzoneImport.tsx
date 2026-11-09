import { plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { lstat } from '@tauri-apps/plugin-fs';
import { useEffect, useState } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import useInvalidate from '../hooks/useInvalidate';
import toastManager from '../lib/toast-manager';
import { logAndNotifyError } from '../lib/utils';

export default function DropzoneImport() {
  const { t } = useLingui();

  const [isShown, setIsShown] = useState(false);
  const invalidate = useInvalidate();

  // Simplification welcome
  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    async function attachFileDropEvent() {
      const unlisten = getCurrentWindow()
        .onDragDropEvent(async (event) => {
          if (event.payload.type === 'over') {
            setIsShown(true);
          } else if (event.payload.type === 'drop') {
            setIsShown(false);

            // Museeks does not deal in terms of files anymore, so we need to only retain folders.
            // Why? Because in case a user imports a specific file from within a folder, it should
            // ignore all other files, but it cannot do that as of today.
            const fileInfos = await Promise.all(
              event.payload.paths.map(async (path) => {
                return {
                  ...(await lstat(path)),
                  path,
                };
              }),
            );

            const folders = fileInfos
              .filter((fileOrFolder) => fileOrFolder.isDirectory)
              .map((folderInfo) => folderInfo.path);

            const skippedItemsCount =
              event.payload.paths.length - folders.length;

            if (skippedItemsCount !== 0) {
              const message = plural(skippedItemsCount, {
                one: '# invalid item ignored',
                other: '# invalid items ignored',
              });

              toastManager.add('warning', message);
            }

            try {
              await LibraryAPI.importTracksFromFolders(folders);
              invalidate();
            } catch (err) {
              logAndNotifyError(err);
            }
          } else {
            setIsShown(false);
          }
        })
        .catch(logAndNotifyError);

      return unlisten;
    }

    const unlisten = attachFileDropEvent();

    return function cleanup() {
      void unlisten.then((unlisten) => (unlisten ? unlisten() : null));
    };
  }, [invalidate, t]);

  if (!isShown) {
    return null;
  }

  return (
    <div {...stylex.props(styles.dropzone)}>
      <div {...stylex.props(styles.dropzoneContent)}>
        {t`Drop folders anywhere to import them`}
      </div>
    </div>
  );
}

const styles = stylex.create({
  dropzone: {
    position: 'fixed',
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
    zIndex: 10000,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    pointerEvents: 'none',
  },
  dropzoneContent: {
    padding: '20px 40px',
    backgroundColor: 'var(--background)',
    borderWidth: '2px',
    borderStyle: 'dashed',
    borderColor: 'var(--primary)',
    borderRadius: '8px',
    fontSize: '18px',
    color: 'var(--text)',
  },
});
