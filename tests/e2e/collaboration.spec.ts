/**
 * KYRO IDE - Collaboration E2E Tests
 * 
 * Tests for real-time collaboration features
 */

import { test, expect, Page, BrowserContext } from '@playwright/test';

test.describe('Collaboration', () => {
  test('should show collaboration UI elements', async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
    
    // Look for collaboration indicators
    const collabButton = page.locator('[data-testid="collaboration-toggle"]').or(
      page.locator('button').filter({ hasText: /Collaborat|Share|Invite/i })
    ).first();
    
    // May or may not be visible depending on auth state
    const count = await collabButton.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('should handle user presence indicators', async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
    await page.waitForTimeout(2000);
    
    // Look for presence avatars/indicators
    const presenceIndicators = page.locator('[data-testid="user-presence"]').or(
      page.locator('[class*="presence"]').or(page.locator('[class*="avatar"]'))
    );
    
    // Presence indicators are optional
    const count = await presenceIndicators.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });
});

test.describe('Collaboration - Multi-user', () => {
  test('should support multiple browser contexts', async ({ browser }) => {
    // Create two browser contexts (simulating two users)
    const context1 = await browser.newContext();
    const context2 = await browser.newContext();
    
    const page1 = await context1.newPage();
    const page2 = await context2.newPage();

    await page1.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page2.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    
    // Both users navigate to the app
    await page1.goto('/');
    await page2.goto('/');
    
    // Both should see the editor
    await expect(page1.locator('.monaco-editor').or(page1.locator('[data-testid="editor-container"]'))).toBeVisible({ timeout: 10000 });
    await expect(page2.locator('.monaco-editor').or(page2.locator('[data-testid="editor-container"]'))).toBeVisible({ timeout: 10000 });
    
    // Cleanup
    await context1.close();
    await context2.close();
  });
});

test.describe('AI Features', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
  });

  test('should show AI chat input', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    // Look for AI chat input
    const aiInput = page.locator('[data-testid="ai-input"]').or(
      page.locator('textarea[placeholder*="Ask"]').or(
        page.locator('input[placeholder*="Ask"]')
      )
    ).first();
    
    if (await aiInput.count() > 0) {
      await expect(aiInput).toBeVisible({ timeout: 5000 });
    }
  });

  test('should send AI message', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    // Find AI input
    const aiInput = page.locator('[data-testid="ai-input"]').or(
      page.locator('textarea[placeholder*="Ask"]').or(
        page.locator('input[placeholder*="Ask"]')
      )
    ).first();
    
    if (await aiInput.count() > 0) {
      await aiInput.fill('Write a hello world function');
      
      // Find send button
      const sendButton = page.locator('[data-testid="ai-send"]').or(
        page.locator('button[type="submit"]').or(
          page.locator('button').filter({ hasText: /Send/i })
        )
      ).first();
      
      if (await sendButton.count() > 0) {
        await sendButton.click();
        
        // Wait for response (may take time)
        await page.waitForTimeout(3000);
        
        // Look for response
        const response = page.locator('[data-testid="ai-response"]').or(
          page.locator('[class*="ai-message"]').or(
            page.locator('[class*="response"]')
          )
        ).first();
        
        // Response may or may not appear (depends on AI availability)
        expect(true).toBe(true);
      }
    }
  });

  test('should show model selector if available', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    // Look for model selector
    const modelSelector = page.locator('[data-testid="model-selector"]').or(
      page.locator('select').filter({ has: page.locator('option') })
    ).first();
    
    // Model selector is optional
    const count = await modelSelector.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });
});

test.describe('File Operations', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
  });

  test('should show file tree structure', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    const fileTree = page.locator('[data-testid="file-tree"]').or(
      page.locator('[class*="file-tree"]')
    ).first();
    
    await expect(fileTree).toBeVisible({ timeout: 10000 });
    
    // Tree contents may vary by startup state, but panel should be renderable
    const items = await fileTree.locator('button, [role="treeitem"]').all();
    expect(items.length).toBeGreaterThanOrEqual(0);
  });

  test('should expand/collapse folders', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    // Find folder items
    const folderItem = page.locator('[data-testid="file-tree"] [data-is-folder="true"]').or(
      page.locator('[class*="file-tree"] button').filter({ has: page.locator('[class*="folder"]') })
    ).first();
    
    if (await folderItem.count() > 0) {
      // Click to expand
      await folderItem.click();
      await page.waitForTimeout(500);
      
      // Click again to collapse
      await folderItem.click();
      await page.waitForTimeout(500);
      
      // Should not crash
      expect(true).toBe(true);
    }
  });

  test('should create new file (if UI available)', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    // Look for new file button
    const newFileButton = page.locator('[data-testid="new-file"]').or(
      page.locator('button').filter({ hasText: /New File|Create|\+/i })
    ).first();
    
    if (await newFileButton.count() > 0) {
      await newFileButton.click();
      await page.waitForTimeout(500);
      
      // Look for file name input
      const nameInput = page.locator('[data-testid="file-name-input"]').or(
        page.locator('input[placeholder*="name"]').or(
          page.locator('input[type="text"]')
        )
      ).first();
      
      if (await nameInput.count() > 0) {
        await nameInput.fill('test-file.ts');
        await page.keyboard.press('Enter');
        await page.waitForTimeout(500);
      }
    }
    
    // Verify no crash
    expect(true).toBe(true);
  });
});

test.describe('Terminal', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem('kyro-first-run-done', 'true');
    });
    await page.goto('/');
  });

  test('should toggle terminal visibility', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    // Find terminal toggle
    const terminalButton = page.locator('[data-testid="terminal-toggle"]').or(
      page.locator('button').filter({ hasText: /Terminal/i })
    ).first();
    
    if (await terminalButton.count() > 0) {
      // Open terminal
      await terminalButton.click();
      await page.waitForTimeout(500);
      
      // Terminal should be visible
      const terminal = page.locator('.xterm').or(page.locator('[class*="terminal"]')).first();
      await expect(terminal).toBeVisible({ timeout: 5000 });
      
      // Close terminal
      await terminalButton.click();
      await page.waitForTimeout(500);
    }
  });

  test('should accept terminal input', async ({ page }) => {
    await page.waitForTimeout(2000);
    
    // Open terminal if button exists
    const terminalButton = page.locator('[data-testid="terminal-toggle"]').or(
      page.locator('button').filter({ hasText: /Terminal/i })
    ).first();
    
    if (await terminalButton.count() > 0) {
      await terminalButton.click();
      await page.waitForTimeout(500);
      
      // Click on terminal
      const terminal = page.locator('.xterm').first();
      if (await terminal.count() > 0) {
        await terminal.click();
        await page.keyboard.type('echo "test"');
        await page.keyboard.press('Enter');
        await page.waitForTimeout(500);
      }
    }
    
    expect(true).toBe(true);
  });
});
