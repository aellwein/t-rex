### Title
title = Учёт времени

### Navigation
nav-calendar = Календарь
nav-projects = Проекты
nav-absences = Отсутствия
nav-settings = Настройки

### Theme

### Common
loading = Загрузка…
loading-failed = Не удалось загрузить данные
save-failed = Не удалось сохранить
delete-failed = Не удалось удалить
save = Сохранить
add = Добавить
edit = Изменить
delete = Удалить
cancel = Отмена
name = Название
color = Цвет
archived = В архиве
paid = Оплачиваемый

### Projects
projects-title = Проекты
no-projects = Проектов пока нет.

### Absences
absences-title = Виды отсутствий
no-absences = Видов отсутствий пока нет.

### Settings
settings-title = Настройки
theme-label = Тема
theme-system = Системная
language-system = Системный
language-label = Язык
daily-hours-label = Рабочих часов в день
invalid-hours = Введите количество часов от 0 до 24.
settings-saved = Настройки сохранены

### week days
Monday = Пн
Tuesday = Вт
Wednesday = Ср
Thursday = Чт
Friday = Пт
Saturday = Сб
Sunday = Вс

### Months
January = Январь
February = Февраль
March = Март
April = Апрель
May = Май
June = Июнь
July = Июль
August = Август
September = Сентябрь
October = Октябрь
November = Ноябрь
December = Декабрь

###
jump-to-current-day = к сегодняшнему дню

### Time recording and overview
record-title = Учёт времени
record-start = Начало
record-end = Конец
record-duration = Длительность
unit-minutes = Минуты
unit-hours = Часы
record-target = Проект или отсутствие
record-choose = Выберите…
record-added = Запись добавлена
records-of-day = Записи за этот день
no-records = За этот день нет записей времени.
has-records = Есть записи времени
overview-title = Обзор
overview-day = День
overview-month = Месяц
overview-projects = Проекты
overview-absences = Отсутствия
err-start-missing = Укажите время начала.
err-start-invalid = Время начала должно быть в формате 09:30.
err-end-invalid = Время окончания должно быть в формате 17:00.
err-duration-missing = Укажите время окончания или длительность.
err-duration-invalid = Длительность должна быть положительным числом.
err-end-before-start = Конец должен быть позже начала.
err-duration-mismatch = Время окончания и длительность не совпадают.
err-past-midnight = Запись времени должна заканчиваться в тот же день.
err-overlap = Это пересекается с другой записью времени в этот день.
err-target-missing = Выберите проект или отсутствие.
record-updated = Запись обновлена
settings-show-email = Показывать мой e-mail в заголовке (по умолчанию: имя)
settings-break-label = Неоплачиваемый перерыв в записи, в минутах (по умолчанию 45)
record-break = Перерыв
err-break-invalid = Перерыв должен быть числом минут от 0 до 1440.

### Reports
nav-reports = Отчёты
report-title = Отчёты
report-by-project = По проектам
report-by-month = По месяцам
report-by-year = По годам
report-export-pdf = Экспорт в PDF
report-empty = Записей времени пока нет.
report-work = Работа
report-absences = Отсутствия
report-break = Перерыв (неоплачиваемый)
report-total = Итого
report-period = Месяц
report-line = Проект / отсутствие
err-break-too-long = Перерыв длиннее времени от начала до конца.
record-now = Сейчас
settings-carry-over-label = Перенос переработки, Ч:ММ, может быть отрицательным (например -1:30 или +12:00)
invalid-carry-over = Введите перенос в формате Ч:ММ, например +12:30 или -1:45.
report-carry-over = Перенос
report-work-period = Работа за период
report-balance = Текущая переработка
report-balance-title = План и факт
report-overtime = Переработка
report-target-hours = План
report-actual-hours = Факт
report-running = Нарастающим итогом
report-by-day = По дням
report-date = Дата
report-difference = Разница
overview-running = Текущая переработка
report-balance-note = Норма: рабочие дни до сегодня × дневная норма. Праздники рабочими днями не считаются. Факт: проектная работа плюс оплачиваемое отсутствие. Текущий: перенос плюс сверхурочные до конца периода.
nav-holidays = Праздники
holidays-title = Праздничные дни
holidays-note = Дни без рабочего времени. Они не входят в норму и во все отчёты.
holidays-empty = В этом году праздников нет.
holidays-date = Дата
holidays-add = Добавить праздник
holidays-import-title = Импорт из календаря
holidays-import-hint = Адрес файла iCalendar (.ics). Его события на весь день добавляются как праздники; уже внесённые дни сохраняют своё название.
holidays-url = Адрес календаря
holidays-import = Импортировать
holidays-import-done = Добавлено: { $added }, уже были: { $existing }.
holidays-import-failed = Импорт не удался
holidays-invalid-date = Введите корректную дату.
holiday-label = Праздник
holidays-paste-title = Вставить файл календаря
holidays-paste-hint = Откройте файл .ics в текстовом редакторе и вставьте сюда его содержимое целиком.
holidays-paste-button = Импортировать вставленный файл
nav-tokens = Токены
tokens-title = Токены командной строки
tokens-note = Токены позволяют клиенту командной строки входить в систему. Отзыв токена выходит из системы на компьютере, который его использует.
tokens-empty = Токенов пока нет. Выполните `t-rex login`, чтобы создать токен.
tokens-created = Создан
tokens-revoke = Отозвать
tokens-revoke-confirm = Отозвать этот токен? Компьютер, который его использует, выйдет из системы.
tokens-expires = Истекает
tokens-expired = Истёк
