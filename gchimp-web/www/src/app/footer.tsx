import styles from "./page.module.css";
import Image from "next/image";

export const Footer = () => {
  return <footer className={styles.footer}>
    <a
      href="https://github.com/khanghugo/gchimp"
      target="_blank"
      rel="noopener noreferrer"
    >
      <Image
        aria-hidden
        src="/github-mark-white.svg"
        alt="File icon"
        width={16}
        height={16}
      />
      Check gchimp on GitHub
    </a>
    <a
      href="https://github.com/khanghugo/gchimp/releases"
      target="_blank"
      rel="noopener noreferrer"
    >
      <Image
        aria-hidden
        src="/window.svg"
        alt="File icon"
        width={16}
        height={16}
      />
      Want to try native gchimp?
    </a>
  </footer>
}