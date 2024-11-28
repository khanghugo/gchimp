'use client'

import { WaveLoop } from "@/programs/wave-loop";
import styles from "./page.module.css";
import { ResMake } from "@/programs/resmake";

export const Main = () => {
    return <main className={styles.main}>
        <div className={styles.programs}>
            <WaveLoop />
            <ResMake />
        </div>
    </main>
}