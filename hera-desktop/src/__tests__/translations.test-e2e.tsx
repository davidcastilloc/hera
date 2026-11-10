import { expect, test } from 'vite-plus/test';
import { page } from 'vite-plus/test/browser';

import { beforeEachSetup, getMainNavigation } from './e2e-helpers';

beforeEachSetup();

test('The language selector should update the UI', async () => {
  await expect
    .element(
      page.getByRole('heading', {
        name: 'Tu biblioteca de cabina está vacía',
      }),
    )
    .toBeVisible();

  await getMainNavigation().getByRole('link', { name: 'Ajustes' }).click();
  await page.getByRole('link', { name: 'Interfaz' }).click();
  await page
    .getByRole('combobox', { name: 'Language' })
    .selectOptions('Français (French)');

  await expect
    .element(page.getByRole('combobox', { name: 'Langue' }))
    .toBeVisible();
  await expect
    .element(page.getByRole('combobox', { name: 'Thème' }))
    .toBeVisible();
});
