# GitHub Container Registry (GHCR) Setup Instructions

If you're getting a `403 Forbidden` error when pushing to GHCR, follow these steps:

## Step 1: Grant Repository Access to the Package

**This is the most common cause of 403 errors!**

1. Go to your package page: https://github.com/users/scottdkey/packages/container/pia-vpn
   - If the package doesn't exist yet, it will be created on first successful push
   - If you see "This package does not exist", that's fine - it will be created automatically
   
2. If the package exists, click **"Package settings"** (on the right sidebar)
3. Scroll down to **"Manage Actions access"** section
4. Click **"Add repository"** button
5. In the dropdown, select your repository: `scottdkey/homelab`
6. Set the role to **"Write"** (this is critical!)
7. Click **"Add repository"** to save

**Note**: If you don't see "Package settings", the package may not exist yet. Try running the workflow first - if it still fails with 403, the package was created but needs access granted.

## Step 2: Verify Repository Workflow Permissions

1. Go to your repository: https://github.com/scottdkey/homelab
2. Click **Settings** → **Actions** → **General**
3. Scroll down to **"Workflow permissions"**
4. Ensure **"Read and write permissions"** is selected (not "Read repository contents and packages permissions")
5. Check **"Allow GitHub Actions to create and approve pull requests"** if you want PR automation
6. Click **"Save"**

## Step 3: Verify Organization Settings (if applicable)

If your repository is part of an organization:

1. Go to your organization settings
2. Navigate to **Actions** → **General**
3. Ensure workflow permissions are not restricted
4. Check that packages can be published

## Step 4: Retry the Workflow

After completing the above steps, retry your workflow run. The package should now be able to push successfully.

## Troubleshooting

If you still get 403 errors after following these steps:

1. **Check if the package exists**: If the package doesn't exist yet, it should be created automatically on first push. If it already exists from a previous manual push, you must grant access as described above.

2. **Verify token permissions**: The workflow uses `github.token` which should have the correct permissions if repository settings are configured correctly.

3. **Check package visibility**: Ensure the package visibility allows your repository to access it.

4. **Organization restrictions**: If this is an organization repository, check if there are organization-level restrictions on package publishing.
