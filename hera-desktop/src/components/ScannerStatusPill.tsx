import React from 'react';

export interface ScannerStatusPillProps {
  fastDone: number;
  fullDone: number;
  total: number;
  sortFrozen: boolean;
  onUnfreezeSort?: () => void;
}

export const ScannerStatusPill: React.FC<ScannerStatusPillProps> = ({
  fastDone,
  fullDone,
  total,
  sortFrozen,
  onUnfreezeSort,
}) => {
  if (total === 0) return null;

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12 }}>
      <span
        style={{
          padding: '2px 8px',
          borderRadius: 999,
          background: 'rgba(20, 184, 166, 0.14)',
          color: '#14b8a6',
          fontWeight: 600,
        }}
      >
        ⚡ FAST {fastDone}/{total} · FULL {fullDone}/{total}
      </span>
      {sortFrozen && (
        <button
          type="button"
          onClick={onUnfreezeSort}
          style={{
            padding: '2px 8px',
            borderRadius: 999,
            border: '1px solid rgba(250, 204, 21, 0.4)',
            background: 'rgba(250, 204, 21, 0.12)',
            color: '#facc15',
            cursor: 'pointer',
            fontSize: 11,
          }}
        >
          Orden congelado · Actualizar
        </button>
      )}
    </div>
  );
};
