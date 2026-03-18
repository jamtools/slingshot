import {invoke} from '@tauri-apps/api/core';

export type BounceQueueStatus = 'queued' | 'running' | 'succeeded' | 'failed';

export interface BounceOptions {
    project_path: string;
    output_name?: string;
    output_format?: 'aiff' | 'wav' | 'mp3';
}

export interface BounceResult {
    success: boolean;
    bounce_path?: string;
    error_message?: string;
}

export const isGarageBandProject = (filePath: string): boolean => {
    const lower = filePath.toLowerCase();
    return lower.endsWith('.band') || lower.endsWith('.band/');
};

export async function autoBounceGarageBand(options: BounceOptions): Promise<BounceResult> {
    return await invoke<BounceResult>('plugin:slingshot|auto_bounce_garageband', {options});
}

export async function checkGarageBandAvailable(): Promise<boolean> {
    return await invoke<boolean>('plugin:slingshot|check_garageband_available');
}
