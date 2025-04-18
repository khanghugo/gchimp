import { ChangeEvent, createRef, FormEvent, useEffect, useState } from "react";
import { GchimpProgram } from "..";

import "./styles.css";
import { loop_wave } from "gchimp-web";
import { UploadButton } from "@/components/upload-button";
import { LabelledCheckBox } from "@/components/labelled-checkbox";
import { RadioGroup } from "@/components/radio-group";

export const WaveLoop = () => {
    const [name, setName] = useState<string | undefined>(undefined);
    const [file, setFile] = useState<File | null>(null);
    // const [status, setStatus] = useState<string>("Status: Idle");
    const [output, setOutput] = useState<Uint8Array | null>(null);
    const [loop, setLoop] = useState<boolean>(true);
    const [sixteenBit, setSixteenBit] = useState<boolean>(true);

    const bitDepthOptions = [
        { value: '8bit', label: '8 bit' },
        { value: '16bit', label: '16 bit' },
    ];

    const submitButton = createRef<HTMLInputElement>();

    const runWaveLoop = async (e: FormEvent<HTMLFormElement>) => {
        // dont refresh
        e.preventDefault();

        // reading the file to byte then pass it to wave_loop
        const reader = new FileReader();

        reader.onload = (e) => {
            const res = loop_wave(new Uint8Array(e.target?.result as ArrayBuffer), loop, sixteenBit);
            setOutput(res);
        };

        if (!file) {
            // setStatus("No file selected")
            return;
        }

        reader.readAsArrayBuffer(file as Blob);
    };

    const changeFile = (e: ChangeEvent<HTMLInputElement>) => {
        const file = (e.target as HTMLInputElement).files?.item(0);
        // the path will be sandboxed so we only care about the file stem
        setName(file?.name);
        setFile(file ? file : null);
    }

    const onDrop = (e: React.DragEvent<HTMLElement>) => {
        e.preventDefault();

        const file = e.dataTransfer.files.item(0);

        setName(file?.name);

        setFile(file ? file : null);
    }

    const getLoopedWave = () => {
        if (!output)
            return;

        // tried and true method
        const blob = new Blob([output], { type: 'audio/wave' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');

        link.href = url;

        console.assert(name, "no file name");
        if (name)
            link.download = `${extract_file_name(name)}_loop.wav`;

        link.click();

        link.remove();
    }

    // when new file is selected, run the program right away
    useEffect(() => {
        // check the files
        if (!name || (name && !name.endsWith(".wav")) || !file || !submitButton.current) {
            setName(undefined);
            setFile(null);
            setOutput(null);
            return
        }

        // equivalent to clicking the run button
        submitButton.current?.click();
    }, [
        file, submitButton, name
    ]);

    // when settings is changed, reset everything
    useEffect(() => {
        setName(undefined);
        setFile(null);
        setOutput(null);
    }, [loop, sixteenBit]);

    return <GchimpProgram name="Wave Loop" className={`wave-loop`} onDrop={onDrop} >
        <form onSubmit={async (e) => runWaveLoop(e)}>
            <LabelledCheckBox label="Loop" id="should-loop" checked={loop} onChange={e => setLoop(e.target.checked)} />
            <RadioGroup
                label="Bit Depth:" // Pass the label for the group
                name="bit-depth" // Pass a name for the radio group
                options={bitDepthOptions} // Pass the array of options
                value={sixteenBit ? '16bit' : '8bit'} // Pass the current value (as string)
                onChange={(selectedValue) => setSixteenBit(selectedValue === '16bit')} // Handle change (convert string back to boolean)
            />
            <UploadButton label={"Select or Drop WAV"} id={"wave-loop-path"} onChange={(e) => changeFile(e)} fileName={name} />
            <div>
                <input type="submit" ref={submitButton} />
                <button type="button" disabled={output === null} onClick={getLoopedWave}><h2>Get looped WAV</h2></button>
            </div>
            {/* <textarea readOnly={true} value={status} /> */}
        </form>
    </GchimpProgram>
}

// input is usually `C:\fake_folder\wave_wave.wav`
// remember front slash like windows
const extract_file_name = (s: string): string => {
    const splits = s.split("\\");
    const stem = splits[splits.length - 1];
    const file_name = stem.split(".")[0];

    return file_name;
}