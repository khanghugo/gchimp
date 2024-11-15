import styles from "./page.module.css";
import { Footer } from "./footer";
import { Main } from "./main";
import { Header } from "./header";

export default function Home() {
  return (
    <div className={styles.page}>
      <Header />
      <Main />
      <Footer />
    </div>
  );
}
