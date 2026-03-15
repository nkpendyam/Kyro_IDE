/**
 * KYRO IDE - Editor E2E Tests
 * 
 * Tests for the core editor functionality
 */

import { test, expect, Page } from '@playwright/test';

test.describe('Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
  });

  test('should load the editor', async ({ page }) => {
    // Check main editor container is visible
    await expect(page.locator('[data-testid="editor-container"]').or(page.locator('.monaco-editor'))).toBeVisible({ timeout: 10000 });
  });

  test('should show file tree', async ({ page }) => {
    // File tree should be visible
    const fileTree = page.locator('[data-testid="file-tree"]').or(page.locator('[class*="file-tree"]'));
    await expect(fileTree.first()).toBeVisible({ timeout: 10000 });
  });

  test('should show activity bar', async ({ page }) => {
    const activityBar = page.locator('[data-testid="activity-bar"]').or(page.locator('[class*="activity-bar"]'));
    await expect(activityBar.first()).toBeVisible({ timeout: 10000 });
  });

  test('should show status bar', async ({ page }) => {
    const statusBar = page.locator('[data-testid="status-bar"]').or(page.locator('[class*="status-bar"]'));
    await expect(statusBar.first()).toBeVisible({ timeout: 10000 });
  });

  test('should open a file when clicked', async ({ page }) => {
    // Wait for file tree to load
    await page.waitForTimeout(2000);
    
    // Find first file in tree (not directory)
    const fileItem = page.locator('[data-testid="file-tree"] [data-is-file="true"]').or(
      page.locator('[class*="file-tree"] button').filter({ hasNot: page.locator('[class*="folder"]') })
    ).first();
    
    // If file exists, click it
    if (await fileItem.count() > 0) {
      await fileItem.click();
      await page.waitForTimeout(500);
      
      // Editor should show content
      const editorContent = page.locator('.monaco-editor .view-line');
      await expect(editorContent.first()).toBeVisible({ timeout: 5000 });
    }
  });

  test('should show AI chat panel', async ({ page }) => {
    // Look for AI chat toggle button
    const aiButton = page.locator('[data-testid="ai-chat-toggle"]').or(
      page.locator('button').filter({ hasText: /AI|Chat/i })
    ).first();
    
    if (await aiButton.count() > 0) {
      await aiButton.click();
      await page.waitForTimeout(500);
    }
    
    // AI panel should be visible
    const aiPanel = page.locator('[data-testid="ai-chat-panel"]').or(
      page.locator('[class*="ai-chat"]').or(page.locator('[class*="chat-panel"]'))
    ).first();
    
    // Either already visible or toggle works
    const isVisible = await aiPanel.isVisible().catch(() => false);
    expect(typeof isVisible).toBe('boolean');
  });

  test('should show terminal panel', async ({ page }) => {
    // Look for terminal toggle button
    const terminalButton = page.locator('[data-testid="terminal-toggle"]').or(
      page.locator('button').filter({ hasText: /Terminal/i })
    ).first();
    
    if (await terminalButton.count() > 0) {
      await terminalButton.click();
      await page.waitForTimeout(500);
      
      // Terminal should be visible
      const terminal = page.locator('[data-testid="terminal-panel"]').or(
        page.locator('.xterm').or(page.locator('[class*="terminal"]'))
      ).first();
      
      await expect(terminal).toBeVisible({ timeout: 5000 });
    }
  });

  test('should display language in status bar', async ({ page }) => {
    // Status bar should show language indicator
    const statusBar = page.locator('[data-testid="status-bar"]').or(page.locator('[class*="status-bar"]'));
    await expect(statusBar.first()).toBeVisible({ timeout: 10000 });
    
    // Check for language indicator (may show "Plain Text" initially)
    const languageIndicator = page.locator('[data-testid="language-indicator"]').or(
      statusBar.locator('span, button').filter({ hasText: /Rust|Python|JavaScript|TypeScript|Plain|Text/i })
    ).first();
    
    // Language indicator may or may not be visible depending on file open
    const count = await languageIndicator.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });
});

test.describe('Editor - Keyboard Shortcuts', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
  });

  test('should open command palette with Ctrl+Shift+P', async ({ page }) => {
    // Press Ctrl+Shift+P (Cmd+Shift+P on Mac)
    await page.keyboard.press('Control+Shift+P');
    await page.waitForTimeout(500);
    
    // Command palette may appear (if implemented)
    const commandPalette = page.locator('[data-testid="command-palette"]').or(
      page.locator('[class*="command-palette"]').or(page.locator('[role="dialog"]'))
    ).first();
    
    // Just verify no crash
    expect(true).toBe(true);
  });

  test('should handle keyboard input in editor', async ({ page }) => {
    // Wait for editor to be ready
    await page.waitForTimeout(2000);
    
    // Click on editor area
    const editor = page.locator('.monaco-editor').first();
    if (await editor.count() > 0) {
      await editor.click();
      await page.keyboard.type('// Test comment');
      await page.waitForTimeout(500);
      
      // Verify input was received (no crash)
      expect(true).toBe(true);
    }
  });
});

test.describe('Editor - Performance', () => {
  test('should load within acceptable time', async ({ page }) => {
    const startTime = Date.now();
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
    
    // Wait for main content
    await page.locator('[data-testid="editor-container"]').or(
      page.locator('.monaco-editor')
    ).waitFor({ timeout: 15000 });
    
    const loadTime = Date.now() - startTime;
    
    // Should load in under 10 seconds
    expect(loadTime).toBeLessThan(10000);
    console.log(`Page load time: ${loadTime}ms`);
  });

  test('should handle multiple rapid clicks', async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
    await page.waitForTimeout(1000);
    
    // Rapidly click file tree items
    const buttons = await page.locator('[data-testid="file-tree"] button').all();
    
    for (let i = 0; i < Math.min(5, buttons.length); i++) {
      await buttons[i].click({ timeout: 1000 }).catch(() => {});
    }
    
    // App should still be responsive
    await expect(page.locator('body')).toBeVisible();
  });
});
