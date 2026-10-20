// Открыто ли окно «Спросить сайт». Отдельно от самого окна и его данных: кнопка
// в шапке стоит на каждой странице, а окно с логикой ответов грузится только
// тогда, когда его впервые открыли (`SiteAskHost`).
import { interact } from '$lib/analytics';

class SiteAskUi {
  open = $state(false);
  /** Окно уже просили: с этого момента его код загружен и остаётся в памяти. */
  mounted = $state(false);

  show(via: 'button' | 'key') {
    if (this.open) return;
    this.mounted = true;
    this.open = true;
    interact('site_ask_open', via);
  }

  hide() {
    this.open = false;
  }

  toggle(via: 'button' | 'key') {
    if (this.open) this.hide();
    else this.show(via);
  }
}

export const siteAskUi = new SiteAskUi();
