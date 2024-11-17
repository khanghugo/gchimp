import styles from "./page.module.css";
import Image from "next/image";

export const Footer = () => {
  const _buildId = (process.env.GCHIMP_WEB_BUILD_ID as string) || "development";
  const _splits = _buildId.split("\"");
  const buildId = _splits.length == 1 ? _splits[0] : _splits[1];

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
    <small className={styles.buildId}>{`buildID: ${buildId}`}</small>
  </footer>
}