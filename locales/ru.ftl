# Русский. Те же сообщения, что и в en.ftl.

## Menu

app-tagline = Интриги за корону — против компьютера
players = Игроки
player-count = { $count }  (вы + { $pcs } ПК)
strength = Сила компьютера
level-easy = Лёгкий
level-medium = Средний
level-hard = Сложный
speed-label = Скорость: { $value }
speed-normal = Обычная
speed-slow = Медленная
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
log-placed-left = { $player }: карта положена в начало очереди.
log-placed-right = { $player }: карта положена в конец очереди.
log-placed-stack = { $player }: карта положена поверх своей карты.
log-kept-hidden = { $player }: карта остаётся закрытой (влияния на ней: { $influence }).
log-revealed = { $player }: раскрыта карта «{ $card }», получено влияния: { $gained }.
log-gained = { $player }: получено влияния: { $amount }.
log-stole = { $thief }: украдено 1 очко влияния (у кого: { $victim }).
log-copied = { $player }: Оборотень копирует карту «{ $card }».
log-eliminated = { $player }: «{ $card }» устраняет «{ $target }» ({ $owner }) (+1).
log-ambushed = { $player }: «{ $card }» попадает в Засаду ({ $owner }) (+{ $gained } за Засаду).
log-discarded = { $player }: сброшена карта «{ $card }».
log-moved = { $player }: карта ({ $owner }) перемещена на позицию { $position }.
log-error = Ошибка: { $error }

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
