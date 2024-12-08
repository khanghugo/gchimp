'use client'

import { WaveLoop } from "@/programs/wave-loop";
import styles from "./page.module.css";
import { ResMake } from "@/programs/resmake";
import { Dem2Cam } from "@/programs/dem2cam";
import { Bsp2Wad } from "@/programs/bsp2wad";

export const Main = () => {
    return <main className={styles.main}>
        <div className={styles.programs}>
            <WaveLoop />
            <ResMake />
            <Dem2Cam />
            <Bsp2Wad />
        </div>
    </main>
}