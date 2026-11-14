# Српски (ћирилица). Исте поруке као у en.ftl.

## Menu

app-tagline = Сплетке за круну, против рачунара
players = Играчи
player-count = { $count }  (ви + { $pcs } ПЦ)
strength = Снага рачунара
level-easy = Лако
level-medium = Средње
level-hard = Тешко
speed-label = Брзина: { $value }
speed-normal = Нормална
speed-slow = Спора
start-game = Почни игру

## Players

player-you = Ви
player-pc = ПЦ { $number }

## Cards

card-archer = Стрелац
card-archer-text = Елиминишите прву или последњу карту у реду.
card-soldier = Војник
card-soldier-text = Елиминишите суседну карту.
card-spy = Шпијун
card-spy-text = Украдите 1 поен утицаја од играча чија је карта суседна.
card-heir = Наследник
card-heir-text = Добијте 2 поена утицаја ако ниједан други Наследник није откривен.
card-assassination = Атентат
card-assassination-text = Елиминишите било коју карту. Одбаците Атентат.
card-royal_decree = Краљевски указ
card-royal_decree-text = Померите карту било где у реду. Одбаците Краљевски указ.
card-lord = Племић
card-lord-text = Добијте 1 поен утицаја, плус 1 за сваку суседну карту ваше породице.
card-shapeshifter = Мењач облика
card-shapeshifter-text = Копирајте способност суседног откривеног лика.
card-ambush = Заседа
card-ambush-text = Ако је противник елиминише док је скривена: добијте 4 и одбаците нападача. Ако је сами откријете: добијте 1. Одбаците Заседу.
card-conspiracy = Завера
card-conspiracy-text = Добијте двоструко више утицаја него што је на њој. Одбаците Заверу.

## Board

round-of = Рунда { $round } од { $rounds }
menu = Мени
influence = { $amount } { $amount ->
    [one] поен
   *[other] поена
} утицаја
in-hand = { $count } у руци
queue-empty = Ред је празан.
face-down = скривена
hidden = (скривена)
place = Стави
here = овде
move-here = Овде
reveal = Откриј
keep-hidden = Не откривај (+1)
pick-another = Изабери другу карту
your-hand = Ваше карте
pick-to-see = Изаберите карту из руке да бисте видели шта ради.
copying = { $card } (копира: { $copied })

## What to do now

game-is-over = Игра је завршена.
thinking = { $player } размишља…
pick-hand-card = Ваш потез: изаберите карту из руке.
pick-place = Изаберите где ћете ставити карту „{ $card }“. Биће окренута лицем надоле.
reveal-or-not = Ваша карта „{ $card }“ (означена): откријте је или је оставите скривену и додајте јој 1 поен утицаја?
decree-pick-card = Краљевски указ: изаберите карту коју померате.
decree-pick-place = Краљевски указ: изаберите ново место за карту.
pick-target = { $card }: изаберите истакнуту карту.

## Game log. Each line describes one event.

log-round = Почиње рунда { $round }.
log-placed-left = { $player }: карта је стављена на почетак реда.
log-placed-right = { $player }: карта је стављена на крај реда.
log-placed-stack = { $player }: карта је стављена преко сопствене карте.
log-kept-hidden = { $player }: карта остаје скривена (утицај на њој: { $influence }).
log-revealed = { $player }: откривена је карта „{ $card }“, добијено утицаја: { $gained }.
log-gained = { $player }: добијено утицаја: { $amount }.
log-stole = { $thief }: украден 1 поен утицаја (од: { $victim }).
log-copied = { $player }: Мењач облика копира карту „{ $card }“.
log-eliminated = { $player }: „{ $card }“ елиминише „{ $target }“ ({ $owner }) (+1).
log-ambushed = { $player }: „{ $card }“ упада у Заседу ({ $owner }) (+{ $gained } за Заседу).
log-discarded = { $player }: одбачена је карта „{ $card }“.
log-moved = { $player }: карта ({ $owner }) премештена је на позицију { $position }.
log-error = Грешка: { $error }

## Labels on cards while an action is shown

mark-copied = копирана
mark-eliminated = елиминисана
mark-ambush = Заседа! +{ $gained }
mark-discarded = одбачена
mark-moves = помера се

## Game over

you-win = Победили сте!
you-share-win = Делите победу!
winners = Победа: { $names }
and = { " " }и{ " " }
tie-break = При нерешеном резултату побеђује онај коме је остало више карата у реду.
back-to-menu = Назад у мени
