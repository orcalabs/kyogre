# Ais Positions Backup

Automated ais-positions backups uploaded to Azure Blob Storage.

## 1. Configure Azure authentication

### Create the service principal

In the Azure Portal:

1.  Open Microsoft Entra ID → App registrations → New registration.
2.  Name it kyogre-backup.
3.  Leave the default supported account type selected and click Register.
4.  On the app's Overview page, copy:
    - Application (client) ID
    - Directory (tenant) ID
5.  Open Certificates & secrets → Client secrets → New client secret.
6.  Create a secret and immediately copy its Value. You won't be able to see it again.

If your work tenant prevents you from registering applications, ask your Azure administrator to create it for you.

### Give it permission to upload blobs

In the Azure Portal:

1.  Open your Storage Account, kyogrebackup.
2.  Go to Access control (IAM) → Add role assignment.
3.  Select Storage Blob Data Contributor.
4.  Select the kyogre-backup app/service principal and assign the role.

### Store the credentials

1. Create the secrets.env file
2. Put these lines in it, replacing the placeholders with your values:

```
AZURE_STORAGE_ACCOUNT=your-storage-account
AZURE_STORAGE_CONTAINER=your-storage-container

AZURE_TENANT_ID=your-tenant-id
AZURE_CLIENT_ID=your-application-client-id
AZURE_CLIENT_SECRET=your-client-secret-value
```

## 2. Register the systemd units

Update the `User=`, `EnvironmentFile=`, and `ExecStart=` values in `kyogre-backup.service` to match your Linux username and absolute repository path.

Create system-level symlinks:

```bash
sudo ln -sfn "$PWD/kyogre-backup.service" \
  /etc/systemd/system/kyogre-backup.service

sudo ln -sfn "$PWD/kyogre-backup.timer" \
  /etc/systemd/system/kyogre-backup.timer
```

Reload systemd and enable the timer:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now kyogre-backup.timer
```

The unit files remain in Git, while systemd loads them through the symlinks.

Keep the repository at a stable path. If you move the repository, update `ExecStart=` and recreate the symlink if necessary.

## 3. Run a manual test

Trigger a backup immediately:

```bash
sudo systemctl start kyogre-backup.service
```

Inspect the result:

```bash
sudo systemctl status kyogre-backup.service
```

View recent logs:

```bash
sudo journalctl -u kyogre-backup.service -n 100 --no-pager
```

Follow logs live:

```bash
sudo journalctl -fu kyogre-backup.service
```

## 4. Verify the schedule

The timer is configured to run on a schedule.

Check the next scheduled run:

```bash
sudo systemctl list-timers kyogre-backup.timer
```

Validate the calendar expression:

```bash
systemd-analyze calendar '*-*-* 01:00:00'
```

## 5. Common operations

Check timer status:

```bash
sudo systemctl status kyogre-backup.timer
```

Run a backup manually:

```bash
sudo systemctl start kyogre-backup.service
```

Stop future scheduled runs:

```bash
sudo systemctl disable --now kyogre-backup.timer
```

Re-enable the schedule:

```bash
sudo systemctl enable --now kyogre-backup.timer
```

After changing either unit file in the repository:

```bash
sudo systemctl daemon-reload
sudo systemctl restart kyogre-backup.timer
```
