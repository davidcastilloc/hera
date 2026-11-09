import * as stylex from '@stylexjs/stylex';
import type React from 'react';
import { useCallback, useRef } from 'react';
import Keybinding from 'react-keybinding-component';

import LibraryAPI from '../api/LibraryAPI';
import useLibraryStore from '../lib/store';
import { isCtrlKey } from '../lib/utils-events';
import Icon from './Icon';

export default function Search() {
  const search = useLibraryStore((state) => state.search);
  const inputRef = useRef<HTMLInputElement>(null);

  const onClear = useCallback(() => LibraryAPI.search(''), []);
  const onChange = useCallback<React.ChangeEventHandler<HTMLInputElement>>(
    (event) => {
      LibraryAPI.search(event.currentTarget.value);
    },
    [],
  );

  const onFocus = useCallback<React.FocusEventHandler<HTMLInputElement>>(
    (e) => e.currentTarget.select(),
    [],
  );

  const onKey = (e: KeyboardEvent) => {
    if (isCtrlKey(e) && e.key.toLowerCase() === 'f') {
      if (inputRef.current) {
        inputRef.current.select();
      }
    }
  };

  return (
    <div {...stylex.props(styles.container)}>
      <span {...stylex.props(styles.searchIcon)} aria-hidden="true">
        <Icon name="search" size={14} />
      </span>
      <input
        type="text"
        placeholder="Buscar pista, artista, tono o BPM..."
        value={search}
        onChange={onChange}
        onFocus={onFocus}
        onMouseUp={(e) => e.preventDefault()}
        spellCheck={false}
        ref={inputRef}
        aria-label="Buscar en biblioteca"
        {...stylex.props(
          styles.input,
          search.length > 0 && styles.inputNotEmpty,
        )}
      />
      {search.length > 0 && (
        <button
          type="button"
          {...stylex.props(styles.clear)}
          onClick={onClear}
          data-museeks-action
          aria-label="Limpiar búsqueda"
        >
          &times;
        </button>
      )}
      <Keybinding preventInputConflict onKey={onKey} />
    </div>
  );
}

const styles = stylex.create({
  container: {
    position: 'relative',
    width: '248px',
    display: 'flex',
    alignItems: 'center',
  },
  searchIcon: {
    position: 'absolute',
    left: '10px',
    display: 'inline-flex',
    alignItems: 'center',
    color: '#8e98a8',
    pointerEvents: 'none',
    zIndex: 2,
  },
  input: {
    display: 'block',
    fontSize: '12.5px',
    width: '100%',
    paddingBlock: '7px',
    paddingLeft: '30px',
    paddingRight: '26px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    borderRadius: '6px',
    lineHeight: 1.3,
  },
  inputNotEmpty: {
    borderColor: 'rgba(20, 184, 166, 0.5)',
  },
  clear: {
    position: 'absolute',
    right: '6px',
    zIndex: 10,
    fontSize: '15px',
    color: '#8e98a8',
    cursor: 'pointer',
    borderStyle: 'none',
    backgroundColor: 'transparent',
    padding: 0,
    height: '20px',
    aspectRatio: '1 / 1',
    lineHeight: 1,
    textAlign: 'center',
  },
});
