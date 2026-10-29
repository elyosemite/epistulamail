// Composição de DS: espacamento.html (.app-shell--blocks) + carregamento.html (.skeleton, --avatar, --title, linhas
// .thread e leitura em skeleton) + contas.html (.account-rail). Mesma forma da tela pronta, para não haver salto.
// O index.html repete esta marcação em HTML puro para aparecer antes do JavaScript.
import { LIST_DEFAULT } from "../lib/listWidth";
import { ReaderSkeleton } from "../ui/Reader";
import { NavSkeleton } from "../ui/Sidebar";
import { ThreadList, ThreadSkeleton } from "../ui/Thread";
import { AppLayout } from "./AppLayout";
import { ContentColumn } from "./ContentColumn";
import { Header } from "./Header";
import { HeaderBrand } from "./HeaderBrand";
import { NavigationBlock } from "./NavigationBlock";

const ROWS: Array<[string, string]> = [["40%", "70%"], ["30%", "55%"], ["45%", "65%"], ["35%", "80%"], ["25%", "60%"], ["40%", "50%"]];
const COMMANDS = ["7.5rem", "5rem", "5.5rem", "7rem", "9rem", "6rem", "6rem"];

/** Tela de abertura: a casca em blocos com cada componente em carregamento. */
export function AppSkeleton() {
  return (
    <AppLayout listWidth={LIST_DEFAULT} hasReader={false}>
      <nav className="account-rail app-shell__rail" aria-hidden="true">
        {[0, 1, 2].map((i) => (
          <span key={i} className="account-rail__item"><span className="skeleton skeleton--avatar" /></span>
        ))}
      </nav>
      <Header
        brand={<HeaderBrand />}
        search={<div className="appbar__search"><span className="skeleton skeleton--title" style={{ width: "100%" }} /></div>}
        commands={
          <div className="commandbar" aria-hidden="true">
            {COMMANDS.map((w, i) => <span key={i} className="skeleton skeleton--title" style={{ width: w }} />)}
          </div>
        }
      />
      <NavigationBlock>
        <nav className="sidebar app-shell__sidebar" aria-hidden="true"><NavSkeleton /></nav>
        <section className="app-shell__list" aria-label="Carregando o EpistulaMail" aria-busy="true">
          <div className="toolbar"><span className="skeleton skeleton--circle" /></div>
          <div className="app-shell__list-body">
            <ThreadList label="Carregando conversas" busy>
              {ROWS.map(([from, subject], i) => <ThreadSkeleton key={i} from={from} subject={subject} />)}
            </ThreadList>
          </div>
        </section>
      </NavigationBlock>
      <ContentColumn>
        <ReaderSkeleton />
      </ContentColumn>
    </AppLayout>
  );
}
