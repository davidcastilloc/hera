import { expect, test } from 'vite-plus/test';
import { page } from 'vite-plus/test/browser';

import {
  beforeEachSetup,
  getMainNavigation,
  setupScannedLibrary,
} from './e2e-helpers';

beforeEachSetup();

test('Biblioteca toolbar should optimize set spine and export DJ crates', async () => {
  await setupScannedLibrary();

  await expect
    .element(page.getByRole('heading', { name: 'Biblioteca' }))
    .toBeVisible();
  await expect
    .element(page.getByText('3 pistas', { exact: true }))
    .toBeVisible();

  const sortMixButton = page.getByRole('button', { name: 'Ordenar mezcla' });
  await sortMixButton.click();
  await expect
    .element(page.getByRole('button', { name: 'Mezcla ordenada' }))
    .toBeVisible();

  const formatSelect = page.getByRole('combobox', {
    name: 'Programa DJ de destino',
  });
  await formatSelect.selectOptions('Rekordbox');

  const exportButton = page.getByRole('button', { name: 'Exportar' });
  await exportButton.click();
  await expect
    .element(page.getByRole('button', { name: 'Exportado a Rekordbox' }))
    .toBeVisible();
});

test('Agente workspace should support approval toggle, suggestion pills, and prompt execution', async () => {
  await setupScannedLibrary();

  await getMainNavigation().getByRole('link', { name: 'Agente' }).click();

  await expect
    .element(page.getByRole('button', { name: 'Nueva sesión' }))
    .toBeVisible();

  const autoBtn = page.getByRole('button', { name: 'Auto', exact: true });
  await expect.element(autoBtn).toBeVisible();
  await autoBtn.click();

  const confirmBtn = page.getByRole('button', {
    name: 'Confirmar',
    exact: true,
  });
  await expect.element(confirmBtn).toBeVisible();
  await confirmBtn.click();

  const suggestionBtn = page.getByRole('button', {
    name: 'Armar playlist armónica',
  });
  await expect.element(suggestionBtn).toBeVisible();
  await suggestionBtn.click();

  await expect
    .element(page.getByText(/ordenada por llave Camelot y mándala a VirtualDJ/))
    .toBeVisible();
  await expect
    .element(page.getByText(/Playlist armónica preparada con 3 pistas/))
    .toBeVisible();
});

test('Transición view should compute harmonic bridge between Actual and Destino tracks', async () => {
  await setupScannedLibrary();

  await getMainNavigation().getByRole('link', { name: 'Transición' }).click();

  await expect
    .element(page.getByRole('heading', { name: 'Transición' }))
    .toBeVisible();
  await expect.element(page.getByText('Actual')).toBeVisible();
  await expect.element(page.getByText('Destino')).toBeVisible();

  await expect
    .element(page.getByRole('button', { name: 'Guardar playlist' }))
    .toBeVisible();
  await expect
    .element(page.getByRole('button', { name: 'Abrir en Agente' }))
    .toBeVisible();
});

test('Mi Estilo view should display DJ DNA metrics, toggle DJ software, and update preferences', async () => {
  await setupScannedLibrary();

  await getMainNavigation().getByRole('link', { name: 'Mi Estilo' }).click();

  await expect
    .element(page.getByRole('heading', { name: 'Mi Estilo' }))
    .toBeVisible();

  const rekordboxBtn = page.getByRole('button', {
    name: 'Rekordbox',
    exact: true,
  });
  await rekordboxBtn.click();
  await expect.element(rekordboxBtn).toHaveAttribute('aria-pressed', 'true');

  const detailedTonesBtn = page.getByRole('button', {
    name: /Tonos detallados/,
  });
  await detailedTonesBtn.click();
  await expect
    .element(page.getByRole('button', { name: 'Ocultar tonos' }))
    .toHaveAttribute('aria-expanded', 'true');

  const switches = page.getByRole('switch');
  await expect.element(switches.nth(0)).toBeVisible();
  await switches.nth(0).click();
});

test('Modo Mini should morph between Booth Card, HUD Pill, and Full Workspace', async () => {
  await setupScannedLibrary();

  await page.getByRole('button', { name: 'Activar Modo Mini' }).click();

  await expect
    .element(page.getByRole('region', { name: 'Modo Mini HERA' }))
    .toBeVisible();

  await page.getByRole('tab', { name: 'Puente' }).click();
  await page.getByRole('tab', { name: 'Siguientes' }).click();

  await page.getByRole('button', { name: 'Plegar a barra' }).click();
  await expect
    .element(page.getByRole('region', { name: 'Barra Mini HERA' }))
    .toBeVisible();

  await page.getByRole('button', { name: 'Volver a ventana completa' }).click();
  await expect
    .element(getMainNavigation().getByRole('link', { name: 'Biblioteca' }))
    .toBeVisible();
});

test('BETA Onboarding Wizard modal and mixes filter toggle should be operable', async () => {
  await setupScannedLibrary();

  // 1. BETA badge click opens wizard modal
  const betaBadgeBtn = page.getByRole('button', { name: 'BETA' });
  await expect.element(betaBadgeBtn).toBeVisible();
  await betaBadgeBtn.click();

  await expect.element(page.getByRole('dialog')).toBeVisible();
  await expect
    .element(page.getByText('Bienvenido a HERA Studio BETA'))
    .toBeVisible();

  // 2. Navigate wizard steps
  const nextBtn = page.getByRole('button', { name: 'Siguiente' });
  await nextBtn.click();
  await expect
    .element(page.getByText('Escaneo Inteligente & Criterio de Cabina'))
    .toBeVisible();

  await nextBtn.click();
  await expect
    .element(page.getByText('Integración de Cabina en Vivo'))
    .toBeVisible();

  const finishBtn = page.getByRole('button', { name: '¡Comenzar a Mezclar!' });
  await finishBtn.click();

  // 3. Mixes filter buttons should be present in Biblioteca toolbar
  const under10mBtn = page.getByRole('button', { name: 'Pistas (< 10m)' });
  const over10mBtn = page.getByRole('button', { name: 'Mixes (> 10m)' });
  const allBtn = page.getByRole('button', { name: 'Todas' });

  await expect.element(under10mBtn).toBeVisible();
  await expect.element(over10mBtn).toBeVisible();
  await expect.element(allBtn).toBeVisible();

  await over10mBtn.click();
  await allBtn.click();
  await under10mBtn.click();
});
