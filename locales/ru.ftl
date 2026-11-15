# Русский. Те же сообщения, что и в en.ftl.

## Menu

app-tagline = Интриги за корону — против компьютера
players = Игроки
player-count = { $count }  (вы + { $pcs } ПК)
strength = Сила компьютера
level-easy = Лёгкий
level-medium = Средний
level-hard = Сложный
speed-normal = Быстро
speed-slow = Медленно
start-game = Начать игру

## Players

player-you = Вы
player-pc = ПК { $number }

## Cards

card-archer = Лучник
card-archer-text = Устраните первую или последнюю карту очереди.
card-soldier = Солдат
card-soldier-text = Устраните соседнюю карту.
card-spy = Шпион
card-spy-text = Украдите 1 очко влияния у игрока, чья карта соседствует с этой.
card-heir = Наследник
card-heir-text = Получите 2 очка влияния, если не раскрыт ни один другой Наследник.
card-assassination = Убийство
card-assassination-text = Устраните любую карту. Сбросьте Убийство.
card-royal_decree = Королевский указ
card-royal_decree-text = Переместите карту в любое место очереди. Сбросьте Королевский указ.
card-lord = Лорд
card-lord-text = Получите 1 очко влияния и ещё по 1 за каждую соседнюю карту вашей семьи.
card-shapeshifter = Оборотень
card-shapeshifter-text = Скопируйте способность соседнего раскрытого персонажа.
card-ambush = Засада
card-ambush-text = Если соперник устранил её закрытой: получите 4 и сбросьте атакующую карту. Если вы раскрыли её сами: получите 1. Сбросьте Засаду.
card-conspiracy = Заговор
card-conspiracy-text = Получите вдвое больше влияния, чем лежит на этой карте. Сбросьте Заговор.

## Board

round-of = Раунд { $round } из { $rounds }
menu = Меню
influence = { $amount } { $amount ->
    [one] очко
    [few] очка
   *[other] очков
} влияния
in-hand = { $count } в руке
queue-empty = Очередь пуста.
face-down = закрыта
hidden = (скрыта)
place = Положить
here = сюда
move-here = Сюда
reveal = Раскрыть
keep-hidden = Не раскрывать (+1)
pick-another = Выбрать другую карту
your-hand = Ваши карты
pick-to-see = Выберите карту в руке, чтобы узнать, что она делает.
copying = { $card } (копирует: { $copied })

## What to do now

game-is-over = Игра окончена.
thinking = { $player } думает…
pick-hand-card = Ваш ход: выберите карту из руки.
pick-place = Выберите, куда положить карту «{ $card }». Она ляжет закрытой.
reveal-or-not = Ваша карта «{ $card }» (отмечена): раскрыть её или оставить закрытой и добавить на неё 1 очко влияния?
decree-pick-card = Королевский указ: выберите карту, которую нужно переместить.
decree-pick-place = Королевский указ: выберите новое место для карты.
pick-target = { $card }: выберите выделенную карту.

## Game log. Each line describes one event.

log-round = Начинается раунд { $round }.
log-placed-left = { $player }: в начало очереди — { $card }.
log-placed-right = { $player }: в конец очереди — { $card }.
log-placed-stack = { $player }: поверх своей карты ({ $target }) — { $card }.
log-kept-hidden = { $player }: карта не раскрыта — { $card }, влияния на ней: { $influence }.
log-revealed = { $player }: раскрыта карта { $card }.
log-gained = { $player }: { $card } приносит влияния: { $amount }.
log-stole = { $player }: { $card } крадёт 1 очко влияния (у кого: { $victim }, соседняя карта: { $target }).
log-copied = { $player }: { $card } копирует карту { $target } ({ $owner }).
log-eliminated = { $player }: { $card } устраняет карту { $target } ({ $owner }).
log-eliminated-self = { $player }: { $card } устраняет сам себя.
log-ambushed = { $player }: { $card } нападает на карту { $target } ({ $owner }) и попадает в засаду.
log-discarded = { $player }: сброшена карта { $card }.
log-moved = { $player }: { $card } перемещает на позицию { $position }: { $target } ({ $owner }).
log-error = Ошибка: { $error }
log-hidden-card = закрытая карта
log-change = { $player } { $amount }
log-total = Итог раунда:
log-empty = В этом раунде ещё ничего не произошло.
log-open = Журнал
log-title = Журнал игры
log-round-tab = Раунд { $round }
log-close = Закрыть
log-paused = Пока журнал открыт, игра ждёт.

## Labels on cards while an action is shown

mark-copied = скопирована
mark-eliminated = устранена
mark-ambush = Засада! +{ $gained }
mark-discarded = сброшена
mark-moves = перемещается

## Game over

you-win = Вы победили!
you-share-win = Вы делите победу!
winners = Победа: { $names }
and = { " " }и{ " " }
tie-break = При равенстве очков побеждает тот, у кого больше карт осталось в очереди.
back-to-menu = В меню
