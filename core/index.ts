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

export type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

export const isGarageBandProject = (filePath: string): boolean => {
    const lower = filePath.toLowerCase();
    return lower.endsWith('.band') || lower.endsWith('.band/');
};

export function createSlingshotClient(invoke: InvokeFn) {
    return {
        autoBounceGarageBand: (options: BounceOptions): Promise<BounceResult> =>
            invoke<BounceResult>('plugin:slingshot|auto_bounce_garageband', {options}),
        checkGarageBandAvailable: (): Promise<boolean> =>
            invoke<boolean>('plugin:slingshot|check_garageband_available'),
    };
}
