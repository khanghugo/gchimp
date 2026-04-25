import { useState, useEffect } from 'react';
// Import 'init' from the same alias your components use
import init from 'gchimp-web';

// This variable lives OUTSIDE the hook so it persists 
// even if components unmount (Singleton pattern)
let wasmPromise: Promise<any> | null = null;

export const useGChimp = () => {
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<Error | null>(null);

    useEffect(() => {
        const initialize = async () => {
            try {
                if (!wasmPromise) {
                    // This points to public/gchimp_web_bg.wasm
                    // init() MUST be called to set the internal 'wasm' variable
                    wasmPromise = init('/gchimp_web_bg.wasm');
                }

                await wasmPromise;
                setIsLoading(false);
            } catch (err) {
                console.error("Failed to load GChimp WASM:", err);
                setError(err as Error);
                setIsLoading(false);
            }
        };

        initialize();
    }, []);

    // Helper to convert File objects to Uint8Array for Rust
    const fileToBytes = async (file: File): Promise<Uint8Array> => {
        const buffer = await file.arrayBuffer();
        return new Uint8Array(buffer);
    };

    return {
        isLoading,
        error,
        fileToBytes
    };
};