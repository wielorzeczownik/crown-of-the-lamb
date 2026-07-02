import * as api from '@/api';
import { translate } from '@/i18n';
import { showToast } from '@/toast';

const GITHUB_REPO = 'wielorzeczownik/crown-of-the-lamb';
const GITHUB_API = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;

// First byte of a valid ESP32 firmware image
const ESP_IMAGE_MAGIC = 0xe9;

const otaState = { isInitialized: false };

function parseSemver(version: string): [number, number, number] {
  const parts = version.replace(/^v/, '').split('.').map(Number);
  return [parts[0] ?? 0, parts[1] ?? 0, parts[2] ?? 0];
}

function isNewerVersion(versionA: string, versionB: string): boolean {
  const [aMaj, aMin, aPat] = parseSemver(versionA);
  const [bMaj, bMin, bPat] = parseSemver(versionB);
  if (aMaj !== bMaj) return aMaj > bMaj;
  if (aMin !== bMin) return aMin > bMin;
  return aPat > bPat;
}

async function checkGithub(): Promise<void> {
  const statusElement = document.getElementById('ota-github-status')!;
  const downloadWrap = document.getElementById(
    'ota-download-wrap'
  ) as HTMLDivElement;
  const downloadLink = document.getElementById(
    'ota-download-link'
  ) as HTMLAnchorElement;
  const currentElement = document.getElementById('ota-current-version')!;

  statusElement.textContent = translate('otaCheckingGithub');

  let currentVersion = '';
  try {
    currentVersion = await api.getVersion();
    currentElement.textContent = `v${currentVersion}`;
  } catch {
    currentElement.textContent = '?';
  }

  try {
    const response = await fetch(GITHUB_API, {
      headers: { Accept: 'application/vnd.github+json' },
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const release = (await response.json()) as {
      tag_name: string;
      assets: { name: string; browser_download_url: string }[];
    };

    const latestTag = release.tag_name;
    const binAsset = release.assets.find(
      (asset) => asset.name === 'crown-of-the-lamb.bin'
    );

    if (currentVersion && isNewerVersion(latestTag, currentVersion)) {
      statusElement.textContent = `${translate('otaUpdateAvailable')}: ${latestTag}`;
      statusElement.style.color = 'var(--accent)';
      if (binAsset) {
        downloadLink.href = binAsset.browser_download_url;
        downloadLink.textContent = `${translate('otaDownloadLink')} (${binAsset.name})`;
        downloadWrap.hidden = false;
      }
    } else {
      statusElement.textContent = translate('otaUpToDate');
      statusElement.style.color = '';
      downloadWrap.hidden = true;
    }
  } catch {
    statusElement.textContent = translate('otaNoGithub');
    statusElement.style.color = '';
  }
}

export function initOta(): void {
  if (otaState.isInitialized) return;
  otaState.isInitialized = true;

  const fileInput = document.getElementById('ota-file') as HTMLInputElement;
  const uploadButton = document.getElementById(
    'ota-upload'
  ) as HTMLButtonElement;
  const dropZone = document.getElementById(
    'file-drop-zone'
  ) as HTMLLabelElement;
  const fileName = document.getElementById('ota-file-name') as HTMLSpanElement;

  fileInput.addEventListener('change', () => {
    const file = fileInput.files?.[0];
    uploadButton.disabled = !file;
    dropZone.classList.toggle('has-file', !!file);
    fileName.textContent = file?.name ?? '';
  });

  uploadButton.addEventListener('click', () => {
    const file = fileInput.files?.[0];
    if (!file) return;

    void file
      .slice(0, 1)
      .arrayBuffer()
      .then((buffer) => {
        if (new Uint8Array(buffer)[0] !== ESP_IMAGE_MAGIC) {
          showToast(false, translate('otaBadMagic'));
          return;
        }
        void runFirmwareUpload(file);
      });
  });

  void checkGithub();
}

async function runFirmwareUpload(file: File): Promise<void> {
  const uploadButton = document.getElementById(
    'ota-upload'
  ) as HTMLButtonElement;
  const progressWrap = document.getElementById(
    'ota-progress-wrap'
  ) as HTMLElement;
  const progressBar = document.getElementById(
    'ota-progress'
  ) as HTMLProgressElement;

  uploadButton.disabled = true;
  progressWrap.hidden = false;
  progressBar.value = 0;

  try {
    const status = await api.uploadFirmware(
      file,
      (percent) => (progressBar.value = percent)
    );
    if (status >= 200 && status < 300) {
      progressBar.value = 100;
      showToast(true, translate('otaSuccess'));
    } else {
      progressWrap.hidden = true;
      uploadButton.disabled = false;
      showToast(false, `${translate('otaError')} ${status}`);
    }
  } catch {
    progressWrap.hidden = true;
    uploadButton.disabled = false;
    showToast(false, translate('otaError'));
  }
}
