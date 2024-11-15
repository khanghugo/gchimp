'use client'

import { WaveLoop } from "@/programs/wave-loop";
import styles from "./page.module.css";

export const Main = () => {
    return <main className={styles.main}>
        <WaveLoop />
    </main>
}