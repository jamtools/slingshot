import {useCallback, useEffect, useMemo, useState} from 'react';

import {autoBounceGarageBand, BounceQueueStatus, checkGarageBandAvailable, isGarageBandProject} from '../core/index';

type UseGarageBandAutoBounceQueueArgs = {
    onProjectBounced?: (projectPath: string, bouncePath: string) => Promise<void> | void;
};

type RunNowOptions = {
    fromQueue?: boolean;
};

const formatUnknownError = (err: unknown) => {
    if (err instanceof Error) {
        return err.message;
    }

    return String(err);
};

export const useGarageBandAutoBounceQueue = (args?: UseGarageBandAutoBounceQueueArgs) => {
    const [garageBandAvailable, setGarageBandAvailable] = useState<boolean | null>(null);
    const [bouncingProjects, setBouncingProjects] = useState<Set<string>>(new Set());

    const [queue, setQueue] = useState<string[]>([]);
    const [activeQueueProjectPath, setActiveQueueProjectPath] = useState<string | null>(null);
    const [queueStatusByPath, setQueueStatusByPath] = useState<Record<string, BounceQueueStatus>>({});
    const [queueErrorByPath, setQueueErrorByPath] = useState<Record<string, string>>({});

    useEffect(() => {
        checkGarageBandAvailable()
            .then(setGarageBandAvailable)
            .catch(() => setGarageBandAvailable(false));
    }, []);

    const runNow = useCallback(async (projectPath: string, opts?: RunNowOptions): Promise<{bouncePath: string}> => {
        if (!isGarageBandProject(projectPath)) {
            throw new Error('Auto bounce is only available for GarageBand projects');
        }

        if (garageBandAvailable === false) {
            throw new Error('GarageBand is not available on this system');
        }

        setBouncingProjects(prev => new Set([...prev, projectPath]));
        if (opts?.fromQueue) {
            setQueueStatusByPath(prev => ({
                ...prev,
                [projectPath]: 'running',
            }));
        }

        try {
            const result = await autoBounceGarageBand({
                project_path: projectPath,
                output_format: 'aiff',
            });

            if (!(result.success && result.bounce_path)) {
                throw new Error(result.error_message || 'Auto bounce failed');
            }

            await args?.onProjectBounced?.(projectPath, result.bounce_path);

            if (opts?.fromQueue) {
                setQueueStatusByPath(prev => ({
                    ...prev,
                    [projectPath]: 'succeeded',
                }));
                setQueueErrorByPath(prev => {
                    const next = {...prev};
                    delete next[projectPath];
                    return next;
                });
            }

            return {bouncePath: result.bounce_path};
        } catch (err) {
            if (opts?.fromQueue) {
                const message = formatUnknownError(err);
                setQueueStatusByPath(prev => ({
                    ...prev,
                    [projectPath]: 'failed',
                }));
                setQueueErrorByPath(prev => ({
                    ...prev,
                    [projectPath]: message,
                }));
            }

            throw err;
        } finally {
            setBouncingProjects(prev => {
                const next = new Set(prev);
                next.delete(projectPath);
                return next;
            });
        }
    }, [args, garageBandAvailable]);

    const enqueue = useCallback((projectPaths: string[]) => {
        const uniqueCandidates = Array.from(new Set(projectPaths));

        const queueSet = new Set(queue);
        const enqueueablePaths = uniqueCandidates.filter(path => {
            if (!isGarageBandProject(path)) {
                return false;
            }

            return path !== activeQueueProjectPath && !queueSet.has(path);
        });

        if (enqueueablePaths.length === 0) {
            return [] as string[];
        }

        setQueue(prev => [...prev, ...enqueueablePaths]);
        setQueueStatusByPath(prev => {
            const next = {...prev};
            enqueueablePaths.forEach(path => {
                next[path] = 'queued';
            });
            return next;
        });

        return enqueueablePaths;
    }, [activeQueueProjectPath, queue]);

    const clearPending = useCallback(() => {
        const pending = new Set(queue);
        setQueue([]);

        setQueueStatusByPath(prev => {
            const next = {...prev};
            pending.forEach(path => {
                if (next[path] === 'queued') {
                    delete next[path];
                }
            });
            return next;
        });

        setQueueErrorByPath(prev => {
            const next = {...prev};
            pending.forEach(path => {
                delete next[path];
            });
            return next;
        });
    }, [queue]);

    useEffect(() => {
        if (activeQueueProjectPath || queue.length === 0) {
            return;
        }

        const [nextProjectPath, ...rest] = queue;
        setQueue(rest);
        setActiveQueueProjectPath(nextProjectPath);

        runNow(nextProjectPath, {fromQueue: true})
            .catch(() => {})
            .finally(() => {
                setActiveQueueProjectPath(null);
            });
    }, [activeQueueProjectPath, queue, runNow]);

    const queueStatusSummary = useMemo(() => {
        const pending = queue.length;
        const running = activeQueueProjectPath;
        return {pending, running};
    }, [activeQueueProjectPath, queue.length]);

    return {
        garageBandAvailable,
        bouncingProjects,
        runNow,
        enqueue,
        clearPending,
        queue,
        activeQueueProjectPath,
        queueStatusByPath,
        queueErrorByPath,
        queueStatusSummary,
    };
};
