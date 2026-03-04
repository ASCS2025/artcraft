import { useEffect, useState } from "react";
import { Button } from "@storyteller/ui-button";
import {
  AppPreferencesPayload,
  CustomDirectory,
  GetAppPreferences,
  SystemDirectory,
} from "@storyteller/tauri-api";
import { PreferenceName, UpdateAppPreferences } from "@storyteller/tauri-api";
import { open } from "@tauri-apps/plugin-dialog";
import { Label } from "@storyteller/ui-label";
import { DownloadDirectoryReveal } from "@storyteller/tauri-api";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faFolder, faMagnifyingGlass, faRotateLeft } from "@fortawesome/pro-solid-svg-icons";

interface MiscSettingsPaneProps {}

export const MiscSettingsPane = (args: MiscSettingsPaneProps) => {
  const [preferences, setPreferences] = useState<
    AppPreferencesPayload | undefined
  >(undefined);

  useEffect(() => {
    const fetchData = async () => {
      const prefs = await GetAppPreferences();
      console.log("prefs", prefs);
      setPreferences(prefs.preferences);
    };
    fetchData();
  }, []);

  // NB: This might be a complex type.
  const outerDownloadObject = preferences?.preferred_download_directory || {};
  const downloadDirectory =
    "custom" in outerDownloadObject
      ? (outerDownloadObject.custom as string)
      : "";
  const currentDownloadLabel =
    "system" in outerDownloadObject
      ? "System Download Directory"
      : downloadDirectory;

  const reloadPreferences = async () => {
    const prefs = await GetAppPreferences();
    console.log("prefs", prefs);
    setPreferences(prefs.preferences);
  };

  const openDirectoryPicker = async () => {
    let directory = await open({
      multiple: false,
      directory: true,
      defaultPath: downloadDirectory || undefined,
    });
    if (directory === null) {
      return; // User dismissed the dialog choice
    }
    await UpdateAppPreferences({
      preference: PreferenceName.PreferredDownloadDirectory,
      value: {
        custom: directory,
      } as CustomDirectory,
    });
    await reloadPreferences();
  };

  const clearDirectory = async () => {
    await UpdateAppPreferences({
      preference: PreferenceName.PreferredDownloadDirectory,
      value: {
        system: "downloads",
      } as SystemDirectory,
    });
    await reloadPreferences();
  };

  const showDirectory = async () => {
    await DownloadDirectoryReveal();
  };

  const [n8nWebhookUrl, setN8nWebhookUrl] = useState<string>(preferences?.n8n_webhook_url || "");

  useEffect(() => {
    setN8nWebhookUrl(preferences?.n8n_webhook_url || "");
  }, [preferences]);

  const saveN8nWebhookUrl = async () => {
    await UpdateAppPreferences({
      preference: PreferenceName.N8nWebhookUrl,
      value: n8nWebhookUrl || undefined,
    });
    await reloadPreferences();
  };

  return (
    <div className="space-y-6 text-base-fg">
      <div className="space-y-2">
        <Label htmlFor="download-path">Default Download Directory</Label>
        <p className="opacity-80">
          This is where downloads are placed after downloading. The current path
          is:
        </p>
        <div className="py-1.5 px-2 rounded-md mt-1 bg-ui-panel border border-ui-panel-border text-base-fg">
          <pre>{currentDownloadLabel}</pre>
        </div>
      </div>
      <div className="flex gap-2">
        <Button variant="primary" onClick={openDirectoryPicker}>
          <FontAwesomeIcon icon={faFolder} />
          Choose Directory
        </Button>
        <Button variant="destructive" onClick={clearDirectory}>
          <FontAwesomeIcon icon={faRotateLeft} />
          Use Default
        </Button>
        <Button variant="secondary" onClick={showDirectory}>
          <FontAwesomeIcon icon={faMagnifyingGlass} />
          Show Directory
        </Button>
      </div>

      <hr className="border-ui-panel-border" />

      <div className="space-y-2">
        <Label htmlFor="n8n-webhook-url">N8n Webhook URL</Label>
        <p className="opacity-80">
          Set a webhook URL to use the N8n Webhook model for image generation.
        </p>
        <input
          id="n8n-webhook-url"
          type="text"
          value={n8nWebhookUrl}
          onChange={(e) => setN8nWebhookUrl(e.target.value)}
          placeholder="https://your-n8n-instance.com/webhook/..."
          className="w-full py-1.5 px-2 rounded-md bg-ui-panel border border-ui-panel-border text-base-fg"
        />
        <Button variant="primary" onClick={saveN8nWebhookUrl}>
          Save
        </Button>
      </div>
    </div>
  );
};
